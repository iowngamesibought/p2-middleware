# Windows packet path: source implementation, runtime not accepted

2026-10-25. This increment adds owned Rust packet translation, a native capture
binding, transactional routing contract and concrete Windows helper. It does **not** enable the desktop's
Windows gateway button or claim a working Windows gateway.

## Packet contract

`src/packet.rs` scopes forwarded traffic to the enrolled console, selected LAN
interface and the 26 legacy discovery destinations from the tested router profile.
For TCP port 27017, a console SYN establishes a bounded mapping. Its destination
becomes the PC listener; its source port becomes a synthetic port unique within
that helper lifetime. Replies restore the original remote address and console
port. Sequence numbers, acknowledgements and payload bytes remain unchanged.
IPv4 and TCP checksums are recalculated, including options and odd-length data.

This is IP/port translation, not a TCP implementation. Windows must accept the
rewritten packet into the listener's TCP stack. That injection path still needs a
live driver test. No synthetic test proves socket delivery or co-op.

At most 64 active mappings are allowed. Idle mappings expire after 180 seconds;
FIN starts a 30-second closing grace, and RST removes the mapping. SYN retransmits
reuse their mapping; a new initial sequence gets a new synthetic port. Ports are
never recycled within one helper lifetime, preventing late replies from reaching
a new connection. Exhaustion rejects new intercepted flows rather than wrapping.

Only selected legacy CM UDP27017 is dropped to preserve the earlier TCP-fallback
experiment. Actual UDP fallback timing remains unverified. Fragmented TCP/UDP
packets within the selected console/legacy-destination scope are rejected, rather
than partially rewriting fragmented flows; this conservatively includes fragments
whose transport port cannot be established. This is a documented limitation.

Outgoing ICMP redirects from this PC to the selected console are suppressed.
Other protocols and unrelated console/destination/port combinations pass unchanged.
No extra NAT is applied to ordinary traffic. Asymmetric normal replies, same-NIC
forwarding, firewall behavior and WAN gameplay all still require live validation.

## Native binding

`src/windivert.rs` provides a WinDivert 2.2 API binding, address-layout checks,
scoped filter compilation, two capture loops and a separate NETWORK injector.
Forwarded CM packets inject as local inbound; translated replies inject outbound.
Metadata counters contain no packet payload or account identity. Shutdown stops
receives and joins the workers before closing handles.

The binding requires approved SHA256 identities for both the DLL and driver.
There are no approved identities in the current desktop, so it cannot load the
driver. File sharing denies writes/replacement while verified files are held;
DLL dependency lookup is restricted to its folder and System32. Hash review is
not a substitute for platform signature checks. No driver was installed or loaded.

ABI source: https://github.com/basil00/WinDivert/blob/v2.2.2/include/windivert.h
API/direction semantics: https://github.com/basil00/WinDivert/blob/v2.2.2/doc/windivert.html

## Startup and recovery contract

`src/routing.rs` implements activation/recovery against an injected Host trait.
For native gateway activation, driver preflight occurs before any journal,
firewall or forwarding mutation. Version 0.1.5 has a separate explicit setup test
that exercises only those Windows settings and immediately restores them; it
never starts capture. It does not satisfy native gateway or gameplay acceptance.
Intent is recorded before mutations, including operations that change state then
return an error. Recovery preserves previously enabled forwarding and only removes
the exact owned firewall rule. Changed interface identity, conflicting rules or
failed cleanup retain the recovery record and block a fresh activation.

The recovery-model tests use an in-memory host. Version 0.1.2 adds a concrete
WindowsHost and p2-route helper. It uses fixed PowerShell operations with bounded
JSON stdin, no command-text interpolation, hidden windows and 20-second command
timeouts. ActiveStore forwarding applies only to the selected IPv4 interface.
The persistent inbound firewall rule is restricted to the selected interface,
PC address, PS3 address, TCP27017 and the sibling bridge executable. Removal
rechecks its name, group, program, interface, address and port identity.

The protected ProgramData/P2Middleware folder requires Administrators ownership,
a protected ACL allowing only SYSTEM and Administrators, and no reparse point.
An exclusive owner.lock prevents simultaneous helpers. Journals are validated and
bounded; writes flush a new file before atomic replacement. A committed record is
preferred over an incomplete next record, because mutation occurs only after a
successful commit. Malformed records fail closed; cleanup conflicts retain state.
Temporary-file replacement, interrupted writes and exclusive locks were tested.
The privileged folder ACL and actual Windows network mutations were not exercised.

The GUI starts the bridge first and waits for its listening event. The helper
receives only non-secret settings over the private named pipe. EOF requests restoration,
including when the GUI dies; a closed output pipe cannot abort recovery. The
helper is deliberately outside the GUI's kill-on-close worker job. Capture failure
and the 10-minute limit also trigger restoration. A helper crash/power loss can
leave a journal and firewall rule; the explicit recovery control retries cleanup.
Moving the package or changing adapter/rule identity can prevent automatic cleanup;
retain the original package location until recovery succeeds. Preexisting enabled
forwarding is retained. Other administrators changing forwarding during a session
cannot be attributed reliably; this remains a live-validation limitation.

Version 0.1.3 starts only the helper with Windows runas approval. The app and
Steam authorization can remain unprivileged. See WINDOWS_HELPER_CONTROL.md for
peer checks, cancellation, permissions and untested UAC boundaries. Both GUI and
helper reject native start while the
compiled approved-driver identity list is empty. Signature status and pinned
file hashes must pass before any capture activation. These source checks do not
establish that Windows can load or accept the driver.

Microsoft's EnableRouter has reference-counted lifetime cleanup but affects IPv4
forwarding at machine scope. It was researched, not used as a shortcut around
selected-interface scoping. The helper uses selected-interface ActiveStore forwarding instead; this still
requires real recovery and ordinary-traffic validation before deployment.
https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-enablerouter

## Defender observation and execution boundary

The official WinDivert 2.2.2-A archive download was blocked when read. Windows
reported that the file contained a virus or potentially unwanted software. A
read-only Defender query identified ThreatID 451610 as
`Misleading:Win32/Malgent!AMTB`, with action success, inactive status and no threat
execution reported. This is an observed classification, **not** proof of either
maliciousness or a false positive. The archive could not be inspected or hashed.

No alternate archive location/download, extraction workaround, exclusion,
protection change or driver execution was attempted after the explicit block.
The blocked runtime archive is excluded from source snapshots and packages.
Native integration remains pending dependency review and normal load validation.

The desktop now has a read-only readiness report and distinguishes Windows
security-block, missing-file, access-denied and image-validation launch errors.
It does not automatically download/install drivers or alter security settings.

## Remaining runtime work

Review the dependency through the normal security process, then validate filters
against the actual DLL and prove one forwarded TCP handshake reaches the listener.
Exercise the implemented Windows Host, protected storage and supervision under
stop/crash/reboot/adapter-change cases, then repeat ordinary PSN forwarding and
stock-PS3 co-op acceptance. Keep the GUI gated until integration can report readiness.
