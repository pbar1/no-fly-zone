use std::collections::HashSet;
use std::process::Command;

use anyhow::{Context, Result};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType, ToVoid};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::propertylist::CFPropertyList;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_foundation::string::CFString;
use itertools::Itertools;
use system_configuration::dynamic_store::{
    SCDynamicStore, SCDynamicStoreBuilder, SCDynamicStoreCallBackContext,
};
use system_configuration::network_configuration::get_interfaces;
use system_configuration::sys::schema_definitions::kSCPropNetLinkActive;
use tracing::info;
use tracing_subscriber::fmt;

fn main() -> Result<()> {
    let format = fmt::format().compact();
    tracing_subscriber::fmt().event_format(format).init();

    let ethernets = &mut HashSet::new();
    let wifis = &mut HashSet::new();
    let active_ethernets = &mut HashSet::new();
    let active_wifis = &mut HashSet::new();

    let callback_context = SCDynamicStoreCallBackContext {
        callout: callback,
        info: ContextState {
            ethernets,
            wifis,
            active_ethernets,
            active_wifis,
        },
    };

    let store = SCDynamicStoreBuilder::new("no-fly-zone")
        .callback_context(callback_context)
        .build();

    for iface in get_interfaces().iter() {
        let r#type = iface
            .interface_type_string()
            .context("error getting interface type")?
            .to_string();
        let name = iface
            .bsd_name()
            .context("error getting interface bsd name")?
            .to_string();
        let display_name = iface
            .display_name()
            .context("error getting interface display name")?
            .to_string();

        let key = format!("State:/Network/Interface/{}/Link", name);
        let key = CFString::from(key.as_str());
        let active = get_link_state(&store, key).unwrap();

        info!(name, r#type, active, display_name, "found interface");

        if r#type == *"Ethernet" {
            ethernets.insert(name.clone());
            if active {
                active_ethernets.insert(name);
            }
        } else if r#type == *"IEEE80211" {
            wifis.insert(name.clone());
            if active {
                active_wifis.insert(name);
            }
        }
    }

    info!(
        names = ethernets.iter().join(","),
        "found ethernet interfaces"
    );
    info!(names = wifis.iter().join(","), "found wifi interfaces");
    info!(
        names = active_ethernets.iter().join(","),
        "active ethernet interfaces"
    );
    info!(
        names = active_wifis.iter().join(","),
        "active wifi interfaces"
    );

    if !active_ethernets.is_empty() && !active_wifis.is_empty() {
        for name in active_wifis.iter() {
            info!(name, "disabling wifi interface");
            set_wifi_state(name, false);
        }
    }

    let watch_keys: CFArray<CFString> = CFArray::from_CFTypes(&[]);
    // TODO: `en.*` works, but better to match all interfaces and filter on type
    let watch_patterns =
        CFArray::from_CFTypes(&[CFString::from("State:/Network/Interface/(en.*)/Link")]);

    if store.set_notification_keys(&watch_keys, &watch_patterns) {
        info!("registered for notifications of link state");
    } else {
        panic!("unable to register for notifications");
    }

    let run_loop_source = store.create_run_loop_source();
    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&run_loop_source, unsafe { kCFRunLoopCommonModes });

    info!("starting run loop");
    CFRunLoop::run_current();

    Ok(())
}

struct ContextState<'a, 'b, 'c, 'd> {
    ethernets: &'a mut HashSet<String>,
    wifis: &'b mut HashSet<String>,
    active_ethernets: &'c mut HashSet<String>,
    active_wifis: &'d mut HashSet<String>,
}

#[allow(clippy::needless_pass_by_value)]
fn callback(store: SCDynamicStore, changed_keys: CFArray<CFString>, context: &mut ContextState) {
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
            info!(name, active, "interface link state changed");

            // We assume we know all WiFi interfaces ahead of time, and any events that
            // aren't WiFi should be used to activate Ethernet. This is...mostly right.
            // FIXME logic is not robust
            if context.wifis.contains(&name) && active && !context.active_ethernets.is_empty() {
                info!(name, "disabling wifi interface");
                set_wifi_state(&name, !active);
                context.active_wifis.remove(&name);
            } else if !context.wifis.contains(&name) {
                info!("disabling all wifi interfaces");
                for w in context.wifis.iter() {
                    info!(name = w, "disabling wifi interface");
                    set_wifi_state(w, false);
                }
            }
        } else {
            info!(name, "interface was removed");
            context.active_wifis.remove(&name);
            context.active_ethernets.remove(&name);

            if context.active_ethernets.is_empty() {
                info!("no active ethernet interfaces, enabling any wifi interfaces");
                for w in context.wifis.iter() {
                    info!(name = w, "enabling wifi interface");
                    set_wifi_state(w, true);
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
