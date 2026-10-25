# Desktop deployment contract

## Host and game scope

Windows 11 x64 is primary. Windows 10 x64 uses the same candidate binary and still
needs runtime validation. Rust MSVC supports Windows 10+ and winit 0.30.13 documents
Windows 10 testing; those are dependency support, not full application acceptance.

- https://doc.rust-lang.org/stable/rustc/platform-support/windows-msvc.html
- https://docs.rs/winit/0.30.13/winit/platform/windows/
- https://docs.rs/eframe/0.33.3/eframe/

Console: PS3-OFW, expected BLUS30732, user-reported 4.93 / game 1.04,
`p_scheme=PS3-P-1.0`. PC peer initially demo_viewer build5270661. Actual running
disc/region identity is not newly verified by this project setup.

## Preview behavior

The preview normally runs unprivileged. A fixed read-only PowerShell query discovers adapters,
with no user interpolation and a 15-second timeout. Settings contain addresses,
prefix and matchmaking preferences only. Steam HTTP requests begin only on
Confirm PS3. TLS validation remains enabled.

QR challenges stay in memory and update with Steam's responses. Cancellation
interrupts polling waits within 100 ms; an in-flight HTTP request can take its
15-second timeout. Cancelled workers cannot install returned authorizations.
Forget stops the bridge and discards local authorization; it does not claim
Steam-side revocation. Reauthorization discards the previous local approval first.

The integrated gateway uses a sibling Rust worker and private stdin
pipe, with no credentials in process arguments. Only allowlisted friendly events
reach the UI. A kill-on-close Windows Job Object owns the worker. Its existing
10-minute limit remains; this is not yet a long-running gateway service.

## Native Windows gateway: driverless preview

Version 0.1.10 uses native temporary /32 addresses for the fixed 26 legacy CM IPv4
addresses, a single PS3/program/interface/address/TCP27017-scoped firewall rule,
and selected-interface forwarding. Steam authorization and the protocol bridge
remain unelevated. The bridge checks the peer and destination and refuses an
upstream CM that overlaps these temporary addresses. No WinDivert dependency is
loaded; its prior code and blocked approval remain for historical comparison.

Startup cleans native and legacy recorded changes before accepting Confirm PS3.
Reset, disconnect, normal close, parent pipe loss and the bounded session limit
request cleanup. Preexisting forwarding is preserved. Exact address ownership
uses interface LUID, creation timestamp, prefix, origins and SkipAsSource.
The durable receipt is retried on startup. Actual forced-crash/reboot acceptance
is still pending; an interrupted address-creation/identity-write gap is refused
rather than guessed. See NATIVE_GATEWAY.md.

Windows11 setup and immediate reset/close passed with all 26 addresses removed,
zero owned rules, forwarding restored Disabled and both helper processes exited.
This validates settings lifecycle only. The prior PS3 browser exchange proves
TCP delivery, not game traffic. Ordinary PSN/DNS/HTTPS forwarding, legacy UDP-to-TCP
fallback, Steam login and co-op require a new live test. No NAT/relay claim follows.
The address aliases affect this PC's routing to those legacy destinations while
active. DHCP-enabled hosts, Windows10 and adapter changes remain untested.

Restore the PS3's original router gateway after disconnecting, before app close,
PC sleep or shutdown. Continued Internet forwarding after cleanup is not promised.

## Before tester distribution

1. Windows 11 GUI, approval lifecycle and worker tests; repeat on Windows 10.
2. Native TCP delivery and ordinary forwarding; ordinary PSN/DNS/HTTPS preserved.
3. Stop, GUI/helper crash, reboot, adapter change and port-conflict recovery.
4. Local gameplay, map transitions and remote NAT/relay; PS3-PS3 is separate.
5. Build provenance, dependency notices, owner license choice and signing.

No driver installation, forwarding change, VM deployment or new game RUN was
performed for project initialization. Linux VM remains a fallback option.

