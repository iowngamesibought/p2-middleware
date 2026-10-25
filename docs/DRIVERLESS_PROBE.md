# Native address probe, 2026-10-26

This is a developer experiment, not a new UI step or an accepted gateway backend.
A possible driverless backend would temporarily own selected legacy CM addresses
on the LAN interface and let the native TCP stack terminate those connections.
This remains a hypothesis until actual LAN delivery and PS3 protocol tests pass.

Microsoft CreateUnicastIpAddressEntry creates nonpersistent address entries.
The implementation uses that API directly, not New-NetIPAddress, whose documentation
states it disables DHCP on an interface where DHCP is enabled. Absence of a DHCP
setting in this API is not proof of compatibility; compare before/after and test a
DHCP-enabled adapter separately. This host's selected LAN has DHCP Disabled.

References:
- https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-createunicastipaddressentry
- https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_unicastipaddress_row
- https://learn.microsoft.com/en-us/powershell/module/nettcpip/new-netipaddress

The fixed probe address is 192.0.2.254/32 (documentation range), with SkipAsSource
true, manual prefix/suffix, and preferred/valid lifetimes of 45 seconds. It must
not already exist. A verified local helper connection and elevation are required;
the existing protected store lock prevents concurrent network transactions. Any
existing recovery journal blocks the probe. There is no arbitrary-IP input.

The test waits at most eight seconds for duplicate-address detection, binds one
local TCP listener to an OS-selected port, and checks a random 32-byte round trip.
The listener checks the peer address. All sockets have bounded I/O. The temporary
address is removed on success/error; a Drop guard retries removal. Removal checks
interface LUID, creation time when available, prefix, origins and source-selection
flag before deleting. A killed helper loses its guard, so the OS lifetime limits
usability; crash expiration and DHCP-enabled behavior are not yet accepted.

Before/after snapshots compare IPv4 addresses/prefix/origins/SkipAsSource, DHCP,
forwarding, default routes and DNS. No firewall rule, forwarding enable, driver,
external endpoint, Steam credential or PS3 session is involved. Local TCP success
would not prove packets arriving from a real PS3, UDP fallback or normal PSN transit.

Developer launcher: p2-network-check.exe <PS3-LAN-IP>. The address selects a unique
matching LAN, not a destination for any probe traffic. The launcher uses the same
mutually verified helper pipe as the app. End users keep the simple app flow.
Installed 0.1.8 remains unchanged while this experimental source advances to 0.1.9.

## LAN probe and observed results

The developer launcher also accepts `--lan`. It binds TCP27017 on the same fixed
documentation address, accepts only the configured PS3 IP and returns a static
HTTP page. The console temporarily uses the PC as gateway. The listener waits at
most 100 seconds; the address has a 150-second lifetime. A scoped inbound rule
matches the helper program, selected interface, test address/port and console IP.
No forwarding is enabled and no official CM addresses are configured by this test.

A protected `lan-probe.json` records intent before mutation and the OS-created
address timestamp afterward. Cleanup validates address identity and rule ownership
before deletion. Both cleanup operations are attempted, with failed evidence kept;
the next helper startup retries. A crash between address creation and identity
recording can require expiry before cleanup: that boundary is not accepted by this
successful normal-exit test. Finite lifetime is not proof of crash recovery.

On 2026-10-26 the local TCP round trip and the actual PS3 browser exchange both
passed. The user saw `P2 native LAN path works` and confirmed restoring the PS3
gateway afterward. Helper before/after network snapshots matched. Independent
readback showed zero test aliases, zero owned rules, no helper processes and
forwarding/DHCP still Disabled on interface3. See
`validation-driverless-probes-20261026.json`. These observations establish TCP LAN
delivery only; Steam handshake, ordinary forwarded traffic and co-op are next.
