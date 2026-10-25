# Driverless gateway preview, 0.1.10

The main app now uses NativeHost through its existing authenticated helper session.
The user flow remains startup cleanup, PS3 address + Confirm, Steam Guard, connected
gateway and close cleanup. There is no new end-user test wizard or driver install.

The helper owns only the compiled 26-address legacy CM list. It refuses any address
already present on any interface, records intent in its protected store, creates
nonpersistent /32 addresses with SkipAsSource and a 720-second lifetime, records
their OS creation identities and waits for duplicate-address checks. DHCP, DNS,
default routes and forwarding are compared before/after address creation. A single
inbound allow rule permits the selected PS3 to the bridge program, selected LAN,
legacy addresses and TCP27017. Forwarding is enabled only if previously disabled.

The unelevated bridge uses a wildcard TCP27017 listener so it can start before
the aliases exist. Before parsing game data, it checks the configured PS3 peer and
the accepted socket's local destination against the shared legacy list. Its
official upstream must not overlap that list, preventing self-interception. Tokens
stay in the existing memory/private-pipe flow. UDP handling is not introduced:
actual console fallback behavior on this path remains a live-test item.

Reset/Stop/EOF use the same session cleanup contract as before. Native and old
driver-path cleanup are both attempted. Recovery removes only exact matching
firewall/address identities and restores forwarding only on the original interface.
The new receipt is native-gateway.json; a pending newer record takes precedence,
and malformed data blocks guessing. A crash between CreateUnicastIpAddressEntry
and timestamp recording is still an ambiguous boundary. Finite lifetime limits
address usability but does not establish crash/reboot recovery or firewall cleanup.

Validation on 2026-10-26: 90 workspace tests, Clippy with warnings denied, format
and locked offline release passed. Real elevated setup, immediate Reset and Stop
passed through the same session API used by the GUI. Independent readback found
none of the 26 addresses, no owned firewall rules, no helper processes, and DHCP
and forwarding Disabled. DNS/default-route/DHCP preservation checks passed inside
setup. The PS3 kept its normal router gateway; no Steam/game session was used.
See validation-native-gateway-20261026.json. The developer command is
`p2-network-check.exe <PS3-IP> --gateway`; it is not an extra user setup step.

During implementation, tests caught Windows' command-line length limit when the
embedded system script grew. Native operations now use a separate fixed embedded
script. Both scripts receive bounded JSON over stdin; neither interpolates user
input into executable script text. The final full check passed after this fix.

Pending: live Steam login and ordinary PSN transit, gameplay, UDP fallback,
DHCP-enabled adapters, forced crash/reboot, network changes and Windows10. Aliases
temporarily affect host routing to those legacy addresses, not only PS3 traffic.
The current successful setup test does not prove public tester readiness.
