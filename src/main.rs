use std::collections::HashSet;
use std::process::Command;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType, ToVoid};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::propertylist::CFPropertyList;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_foundation::string::CFString;
use system_configuration::dynamic_store::{
    SCDynamicStore, SCDynamicStoreBuilder, SCDynamicStoreCallBackContext,
};
use system_configuration::network_configuration::get_interfaces;
use system_configuration::sys::schema_definitions::kSCPropNetLinkActive;

fn main() {
    let ethernets = &mut HashSet::new();
    let wifis = &mut HashSet::new();
    let active_ethernets = &mut HashSet::new();
    let active_wifis = &mut HashSet::new();

    let callback_context = SCDynamicStoreCallBackContext {
        callout: callback,
        info: Context {
            ethernets,
            wifis,
            active_ethernets,
            active_wifis,
        },
    };

    let store = SCDynamicStoreBuilder::new("no-fly-zone")
        .callback_context(callback_context)
        .build();

    let ifaces = get_interfaces();
    for iface in ifaces.iter() {
        let if_type = iface.interface_type_string().unwrap().to_string();
        let if_name = iface.bsd_name().unwrap().to_string();
        let if_disp = iface.display_name().unwrap();

        let key = format!("State:/Network/Interface/{}/Link", if_name);
        let key = CFString::from(key.as_str());
        let active = get_link_state(&store, key).unwrap();

        println!(
            "found interface: name=\"{}\", type=\"{}\", display_name=\"{}\", active=\"{}\"",
            if_name, if_type, if_disp, active,
        );

        if if_type == String::from("Ethernet") {
            ethernets.insert(if_name.clone());
            if active {
                active_ethernets.insert(if_name);
            }
        } else if if_type == String::from("IEEE80211") {
            wifis.insert(if_name.clone());
            if active {
                active_wifis.insert(if_name);
            }
        }
    }

    println!("found ethernet links: {:?}", ethernets);
    println!("found wifi links: {:?}", wifis);
    println!("found active ethernet links: {:?}", active_ethernets);
    println!("found active wifi links: {:?}", active_wifis);

    if active_ethernets.len() > 0 && active_wifis.len() > 0 {
        for name in active_wifis.iter() {
            println!("disabling wifi interface: name=\"{}\"", name);
            set_wifi_state(&name, false);
        }
    }

    let watch_keys: CFArray<CFString> = CFArray::from_CFTypes(&[]);
    // TODO: `en.*` works, but better to match all interfaces and filter on type
    let watch_patterns =
        CFArray::from_CFTypes(&[CFString::from("State:/Network/Interface/(en.*)/Link")]);

    if store.set_notification_keys(&watch_keys, &watch_patterns) {
        println!("registered for notifications of link state");
    } else {
        panic!("unable to register for notifications of link state");
    }

    let run_loop_source = store.create_run_loop_source();
    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&run_loop_source, unsafe { kCFRunLoopCommonModes });

    println!("entering run loop");
    CFRunLoop::run_current();
}

#[derive(Debug)]
struct Context<'a, 'b, 'c, 'd> {
    ethernets: &'a mut HashSet<String>,
    wifis: &'b mut HashSet<String>,
    active_ethernets: &'c mut HashSet<String>,
    active_wifis: &'d mut HashSet<String>,
}

#[allow(clippy::needless_pass_by_value)]
fn callback(store: SCDynamicStore, changed_keys: CFArray<CFString>, context: &mut Context) {
    for key in changed_keys.iter() {
        let name = key
            .clone()
            .to_string()
            .strip_prefix("State:/Network/Interface/")
            .unwrap()
            .strip_suffix("/Link")
            .unwrap()
            .to_string();
        if let Some(active) = get_link_state(&store, key.clone()) {
            println!(
                "link state changed: name=\"{}\", active=\"{}\"",
                name, active
            );

            if context.wifis.contains(&name) && active && !context.active_ethernets.is_empty() {
                println!("disabling wifi interface: name=\"{}\"", name);
                set_wifi_state(&name, !active);
                context.active_wifis.remove(&name);
            } else if context.ethernets.contains(&name) && active {
                println!("saw activated ethernet link, disabling all wifi links");
                for w in context.wifis.iter() {
                    println!("disabling wifi interface: name=\"{}\"", w);
                    set_wifi_state(&w, false);
                }
            } else if !context.wifis.contains(&name) && !context.ethernets.contains(&name) && !active {
                // Assume it's a new Ethernet link being plugged in, add it
                println!("new inactive ethernet link, adding to list");
                context.ethernets.insert(name);
            }
        } else {
            println!("removed link: {}", name);
            context.active_wifis.remove(&name);
            context.active_ethernets.remove(&name);

            if context.active_ethernets.is_empty() {
                println!("no active ethernet links, enabling any wifi links");
                for w in context.wifis.iter() {
                    println!("enabling wifi interface: name=\"{}\"", w);
                    set_wifi_state(&w, true);
                }
            }
        }
    }
}

fn get_link_state(store: &SCDynamicStore, path: CFString) -> Option<bool> {
    let link_state_dict = store
        .get(path)
        .and_then(CFPropertyList::downcast_into::<CFDictionary>)?;
    let link_state = link_state_dict
        .find(unsafe { kSCPropNetLinkActive }.to_void())
        .map(|ptr| unsafe { CFType::wrap_under_get_rule(*ptr) })
        .and_then(CFType::downcast_into::<CFBoolean>)?;
    Some(link_state.into())
}

fn set_wifi_state(link: &str, active: bool) -> Option<()> {
    let state = match active {
        true => "on",
        false => "off",
    };
    Command::new("/usr/sbin/networksetup")
        .arg("-setairportpower")
        .arg(link)
        .arg(state)
        .status()
        .ok()?;
    Some(())
}
