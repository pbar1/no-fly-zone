# `no-fly-zone` :no_entry_sign: :flight_departure:

macOS daemon that toggles WiFi depending on Ethernet link status.

It watches macOS's System Configuration Framework to determine if both WiFi
and Ethernet links are active. If so, it disables WiFi until Ethernet becomes
deactivated.

The name references _AirPort_, Apple's name for WiFi-related things.

## Notes

- https://apple.stackexchange.com/questions/232359/notify-the-system-that-preferences-were-changed
- https://github.com/mullvad/system-configuration-rs/blob/master/system-configuration/examples/watch_dns.rs
