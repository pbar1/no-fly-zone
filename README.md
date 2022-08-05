# `no-fly-zone` :no_entry_sign: :flight_departure:

macOS daemon that toggles WiFi depending on Ethernet link status.

It watches macOS's System Configuration Framework to determine if both WiFi
and Ethernet links are active. If so, it disables WiFi until Ethernet becomes
deactivated.

The name references _AirPort_, Apple's name for WiFi-related things.

## Example `launchd` LaunchAgent

Put the following in `~/Library/LaunchAgents/org.nixos.no-fly-zone.plist`,
making sure to change the path of the `no-fly-zone` binary.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>org.nixos.no-fly-zone</string>
	<key>ProgramArguments</key>
	<array>
		<string>/bin/sh</string>
		<string>-c</string>
		<string>exec /Users/user/.local/bin/no-fly-zone</string>
	</array>
	<key>StandardErrorPath</key>
	<string>/tmp/no-fly-zone.err</string>
	<key>StandardOutPath</key>
	<string>/tmp/no-fly-zone.out</string>
</dict>
</plist>
```

## Notes

- https://apple.stackexchange.com/questions/232359/notify-the-system-that-preferences-were-changed
- https://github.com/mullvad/system-configuration-rs/blob/master/system-configuration/examples/watch_dns.rs
