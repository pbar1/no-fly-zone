# `no-fly-zone`

macOS daemon that toggles WiFi depending on Ethernet link status.

`no-fly-zone` watches macOS's System Configuration to determine if both WiFi
and Ethernet links are active. If so, it disables WiFi until Ethernet becomes
deactivated.

## Notes

- https://apple.stackexchange.com/questions/232359/notify-the-system-that-preferences-were-changed
- https://github.com/mullvad/system-configuration-rs/blob/master/system-configuration/examples/watch_dns.rs
