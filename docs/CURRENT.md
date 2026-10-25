# Current state

## Latest: v0.1.0 public release candidate (2026-10-26, not published)

User confirmed PS3-PC co-op with the desktop app using a Steam friend invite, then
requested a release. Owner chose MIT (copyright "The p2-middleware authors", no
personal credit), version 0.1.0, driver path removed, router script kept local.
Removed: windivert, packet, driver_review, preflight, lan_probe, alias_probe,
routing, legacy WindowsHost/journal recovery, p2-network-check, unused PowerShell
operations, libloading and sha2. The helper accepts only the Session command.
Old routing.json/lan-probe.json records are no longer read or cleaned.
`cargo check` and a release build (path-remapped) pass with no warnings. Binary
scan found no user-profile paths. **The slimmed build has not been run with the
PS3 yet.** No commit, tag or publish was made; the owner publishes.
Outputs: dist/release (zip, SHA256SUMS.txt, RELEASE_NOTES.md), dist/source.

## Latest: router-redirect to PC terminal bridge (2026-10-26)

`scripts/router-bridge.py` runs `p2-bridge authorize`/`serve` on the PC with a loopback
QR page. While the bridge listens, the OpenWrt router (10.0.0.1, br-lan) holds one
owned `ip p2_pc_bridge` table: PS3 TCP27017 to the 26 legacy CMs is DNATed to the
PC and SNATed to the router, legacy UDP27017 is dropped. The PS3 keeps the router
gateway; other traffic is routed normally. The table is removed on exit or SSH EOF.

Live, with Portal 2 PC demo_viewer build5270661 on the same PC: router rules passed
`nft -c`, Steam QR approval, PS3 connect and official login result1. PS3 Find Partner
searches returned zero rooms; the waiting room was relabelled ps3 to default and a
member joined, but the user reported being matched with other players, not this PC.
Random Find Partner pairing is therefore a race against public players. The user
confirmed that a Steam friend invite sent from the PS3 game menu works. Gameplay
duration, stability and the desktop-app path under invites were not assessed; the
earlier desktop-app pairing failure was likely the same race, but that is unproven.

## Latest: 0.1.10 unbounded gateway session (2026-10-26)

All automated tests were removed at the user's request; the workspace has none.
The 10-minute session cap is removed. Only legacy CM TCP27017 from the selected
PS3 is handled by the bridge; all other PS3 traffic uses Windows forwarding.
The 26 aliases keep a 720-second lifetime as the crash bound, and the helper renews
it every 60 seconds while active; a failed renewal triggers normal cleanup. The
bridge accepts up to 1000 PS3 connections within 24 hours. Only `cargo check
--workspace --locked --offline` was run (clean). No Clippy, release build, package,
live renewal, Steam login, PSN transit or co-op RUN yet. Long-session behavior and
address renewal on Windows remain unverified.

## Latest: 0.1.10 integrated driverless gateway

Confirm PS3 now selects native Windows address delivery, without the blocked
packet driver. Same startup/reset/close helper session and unelevated Steam bridge.
Protected native receipt, fixed legacy addresses, exact peer/destination checks,
scoped firewall rule, preserved forwarding and upstream-overlap rejection added.
See NATIVE_GATEWAY.md for scope and crash boundaries.

90 tests, Clippy, formatting and locked offline release pass. A real elevated
setup → Reset → Stop check passed on Windows11. Independent readback verified all
26 aliases absent, no owned rules/helpers, DHCP and forwarding Disabled. No PS3
gateway change, Steam login or game RUN in that check. Evidence:
validation-native-gateway-20261026.json. Next: updated GUI Steam login test, followed
by ordinary PSN transit and co-op. Crash/reboot, DHCP-enabled hosts and Windows10
remain untested. Earlier 0.1.8 driver-blocked status below is historical.

## Latest: 0.1.9 driverless LAN experiment passed

2026-10-26: native Windows temporary-address TCP delivery passed first locally,
then from the stock PS3 at 10.0.0.220 with temporary gateway 10.0.0.100.
The PS3 browser displayed the expected page at 192.0.2.254:27017; the helper
accepted only the configured console address and completed cleanup. Independent
readback found no test address, no owned firewall rules and no helper processes;
interface3 DHCP and forwarding both remained Disabled. User confirmed restoring
the PS3 gateway afterward. Evidence: validation-driverless-probes-20261026.json.

The experimental developer launcher adds no new main-UI steps. Its LAN probe uses
a fixed documentation address, a scoped firewall rule, a protected cleanup receipt
and finite address lifetime. Next helper startup retries recorded cleanup.
87 workspace tests, Clippy and release build pass. Installed 0.1.8 is unchanged.

This proves a native TCP path from the PS3 browser, not the Steam/game gateway.
Next: integrate temporary legacy CM addresses with the Rust bridge and simple
lifecycle, then validate Steam login, ordinary PSN transit and co-op. DHCP-enabled
hosts, UDP fallback, crash expiry/recovery and Windows10 remain untested. No driver
was loaded, no security protection was disabled and no Steam credentials were used.

## Latest: 0.1.8 simple application lifecycle

Follow-up live validation: startup cleanup and idle close now pass on Windows11.
The identical hash-verified bundle is installed at
C:/Users/24548/AppData/Local/Programs/P2Middleware with a Start menu shortcut.
A network-share launch stalled before an app process appeared; the local copy
launched immediately (share root cause unproven). User approved the helper and
reported Ready; UI independently showed cleanup complete. Normal close exited both
GUI and helper. Readback: zero owned rules, interface3 forwarding Disabled.
This validates idle lifecycle only, not cleanup after active gateway mutation.
Evidence: validation-local-lifecycle-20261026.json. Driver issue remains open and
no sample was restored/downloaded/loaded. No Steam/PS3 RUN or security changes.
The earlier pending-live wording below is the pre-test implementation record.

User-required flow: startup cleanup, PS3 address + confirmation to apply,
and close cleanup. The GUI now presents this flow with automatic LAN selection,
a single Confirm PS3 button, integrated Steam Guard sign-in and Details. Separate
setup tests, probes, manual recovery and external-routing controls are removed
from the main UI. Failed operations retain their specific message.

One separately elevated helper owns the protected store for the app lifetime.
Startup cleanup must succeed before configuration is accepted; Reset disconnects
and restores without another UAC prompt. Normal close waits for cleanup. EOF,
malformed commands and parent loss also restore; cleanup failure retains evidence.
Only app-recorded settings are changed, preserving preexisting forwarding.
New journals record the bridge program path (schema2) so upgrades don't require
an old folder. Schema1 migration reads the exact journal rule's program identity,
then checks its name/group/port/address/interface and expected IPs; never executes
that path. The store lock remains held throughout the session.

84 tests pass, including the helper session with simulated startup leftovers,
confirm/reset/reconfirm/Stop, malformed input, parent disconnect, setup failure,
and cleanup failure blocking setup. Real non-admin IPC rejects Session before
mutation. Upgrade identity, scalar firewall and LAN-selection regressions pass.
Clippy, format and locked offline release pass. GUI smoke mode was visually checked
and exited automatically; it skips elevation and mutation. Real elevated lifecycle
and native packet-path acceptance remain pending. Evidence:
validation-simple-flow-20261026.json. Matching source bundle 0.1.8 and stable app/
copy are prepared; no running old app is replaced in place.

Read-only observation this turn: no P2MiddlewareOwned firewall rules, interface3
forwarding Disabled. This supersedes the prior remaining-rule observation; the
protected journal was not read and successful recovery was not inferred from rule
absence. No new network mutation, driver load, Steam approval or PS3 RUN occurred.
Driver approval is still empty. Confirm reports the dependency blocker rather
than applying an unusable partial gateway. Keep the PS3 gateway unchanged.

## Previous fix: 0.1.7

The v0.1.6 retry reached setup but failed during ownership readback and recovery.
Actual GUI Activity reported recovery incomplete. Read-only inspection found one
scoped owned rule (p2-middleware-a904337d3e5011bf) remaining; interface 3 forwarding
was Disabled and no helper remained. The protected journal was not read by the
unelevated observer; Activity says it was retained. Recovery is still pending.

Windows returned InterfaceAlias, LocalAddress and RemoteAddress as scalar strings.
Indexing these directly selected their first character, causing our own rule to
fail ownership validation. Normalize each field with @() before counting/indexing.
The exact patched read-only query accepts the retained rule and returns both full
addresses. Ownership requirements remain unchanged. A regression executes the
embedded script with read-operation fixtures: scalar and one-element array fields
pass; multiple aliases or addresses fail. All 80 workspace tests, formatting,
Clippy and locked offline release build pass. No new GUI smoke was needed for this
helper-only fix. Evidence: validation-scalar-fix-20261025.json.

Recovery must use the original v0.1.6 folder because journal schema 1 does not
record the original bridge program path. The helper-only repair is now installed
in that folder, with the original helper backed up and hashes verified. Receipt:
helper-repair-receipt-20261025.json. Original source and manifest are preserved;
the original manifest no longer describes the replaced helper. The user now chooses
Recover previous network settings in the currently open app and approves Windows.
Do not start another setup test until recovery succeeds. A separate matching
v0.1.7 development/source bundle is also prepared. Driver approval remains blocked,
native gateway disabled, and the PS3 gateway must remain unchanged. No new game RUN.

## Previous fix: 0.1.6

The user's 0.1.5 setup test stopped at get_rule, before journal creation, firewall
addition or forwarding. The app Activity and user report matched. LAN forwarding
was observed Disabled afterward; the protected directory exists but was not
readable by the unelevated observer. No helper was still running.

Reproduced the exact read-only query: it exited 0 but emitted empty stdout for an
absent rule. Windows PowerShell 5.1 ConvertTo-Json emits nothing for a top-level
null, so Rust treated the empty result as a transport failure. The script now
emits literal JSON null. The same query now returns valid null and a real Windows
read-only regression checks Option<Rule>::None. No permissions were loosened.
Evidence: setup-failure-null-20261025.json.

79 workspace tests, Clippy, formatting, release build and GUI smoke are recorded
in validation-null-fix-20261025.json. Matching source/development package:
dist/p2-middleware-0.1.6-dev-windows-x64/. Next: repeat the same explicit setup and
restore test with this package. Keep the detected adapter/router and console
gateway unchanged. The complete elevated test remains pending; the earlier
read-only administrator helper test remains passed. Driver approval is unchanged.

## Previous increment: 0.1.5

Added an explicit Windows setup-and-restore test, independent of packet capture.
It validates the selected adapter, journals intent, creates the same scoped bridge
firewall rule, enables selected-interface forwarding if previously off, reads back
the resulting state and immediately restores its changes. Preexisting forwarding
is preserved. It never opens a listener, signs into Steam or calls driver preflight,
loading or capture. This is a mutating diagnostic, not the read-only helper probe.
The GUI explains the changes and requests Windows permission only on its test button.

Recovery now verifies the rule is absent and owned forwarding changes are restored
before clearing its journal. A command returning success while leaving state behind
is a failure, retaining recovery evidence. Safe stage-specific failure messages help
identify adapter, firewall and forwarding problems without raw command output.

78 workspace tests pass, including setup/restore without a driver, mutation-failure
rollback, preserved preexisting forwarding and unsuccessful-cleanup readback. The
real non-admin helper rejects CheckSetup before host changes. Clippy, release build,
formatting and GUI smoke results: validation-setup-check-20261025.json. Source bundle:
dist/p2-middleware-0.1.5-dev-windows-x64/.

The real elevated setup test has not yet run. Next user action: open 0.1.5 normally,
select the detected LAN adapter, enter the known PS3 address (10.0.0.220 if unchanged),
expand Test Windows setup and restoration, run its test and approve Windows.
Keep the PS3 gateway unchanged; no Steam approval or running console is needed.
See WINDOWS_SETUP_TEST.md for outcomes and recovery. Driver review remains blocked,
and native gateway start remains disabled. No new PS3 RUN or driver load is claimed.

## Previous increment: 0.1.4

Completed a read-only review of upstream release metadata and the selected local
Defender detections. The blocked item is the WinDivert archive; no new release,
upstream asset digest or maintainer resolution was found. See DRIVER_REVIEW.md
and driver-review-observation-20261025.json. No archive retry, alternate download,
restoration, exception, upload or external message occurred.

The asynchronous readiness check now reports each expected driver file's access,
x64 header, SHA256, signature and configured-identity match separately. It never
loads code or approves an inspected file. Malformed, oversized and wrong-architecture
files fail inspection; security blocks are distinguished from access-denied errors.
The GUI stays responsive during signature queries. Native start remains disabled.

75 tests pass, including malformed-header bounds, inert-file hashing, explicit
security-error classification and real Windows signature inspection of our own
test executable. Clippy, formatting, release build and GUI smoke results are in
validation-driver-review-20261025.json. Matching source bundle:
dist/p2-middleware-0.1.4-dev-windows-x64/.

The user-run 0.1.3 elevated helper check remains passed. Driver identity approval,
privileged networking/cleanup, ordinary PSN traffic and co-op acceptance are still
pending. The dependency review has not resolved the detection. Keep the console's
gateway unchanged; no PS3 is needed for this increment.

## Previous increment: 0.1.3

Separated Windows administrator setup from the normal desktop session. Only
p2-route requests UAC elevation; Steam authorization and the bridge stay in the
ordinary user session when the app is opened normally. The local named-pipe
control channel has explicit permissions, first-instance ownership, remote-client
rejection, process-ID verification at both ends and bounded messages. It carries
no Steam credentials. Stop and GUI disconnect preserve helper-owned cleanup.

Added Check helper (Windows permission): a read-only probe that reports actual
helper elevation without driver loading, recovery storage or network changes.
The GUI stays responsive while Windows asks for permission. PowerShell is now
resolved from the Windows system directory with a system-only module path.
See WINDOWS_HELPER_CONTROL.md for the exact boundary and remaining validation.

70 workspace tests pass: real non-admin helper processes exchange probe/stop
messages, exit on disconnect and reject invalid/unapproved commands. Native pipe
ownership/PID checks, timeout-safe frame bounds and a read-only system-shell
round trip pass. Clippy, formatting, release build and GUI startup/exit checks
are recorded in validation-control-20261025.json. Matching source is included in
dist/p2-middleware-0.1.3-dev-windows-x64/.

User follow-up: the packaged app's Activity panel shows three completed read-only
checks with **Administrator helper connection verified**. Packaged binary hashes
match the build record. Elevated-helper communication is now observed passing;
see validation-helper-user-check-20261025.json. The UAC prompt itself was not
observed by the agent, and the GUI's token level was not independently measured.
Cancellation, alternate-admin credentials and cross-integrity isolation remain
separate tests. The earlier build-time validation record is unchanged.

No driver, elevated network mutation, Steam approval or PS3 session was tested.
The WinDivert archive block remains unresolved; no approved driver hashes exist.
Native start remains disabled. Public signing/installation, real network recovery,
Windows 10 and live co-op acceptance are still pending. Do not change the PS3's
gateway or install the driver yet.

## Previous increment: 0.1.2

Implemented the concrete Windows routing helper and connected it to the desktop:
selected-adapter forwarding, a console/address/program-scoped bridge firewall
rule, protected recovery journal, exclusive ownership and GUI-exit cleanup.
The bridge must report a listening socket before routing starts. A separate
recovery control handles saved state after helper failure. Reporting pipe closure
cannot abort cleanup. See WINDOWS_PACKET_PATH.md and DEPLOYMENT.md for limits.

63 workspace tests pass, including actual temporary-file journal replacement,
exclusive-lock and interrupted-write tests, plus the existing packet/recovery
model tests. Formatting, Clippy with warnings denied, PowerShell syntax check
and release build pass. Release GUI smoke exits 0 with no owned workers left.
Helper status is read-only; native run refuses with driver_unapproved (exit 1),
and no protected journal directory was created. Evidence:
validation-helper-20261025.json. No elevated networking mutation was exercised.

The official WinDivert archive remains blocked by Windows security; the previous
observation is retained in validation-network-20261025.json. No approved driver
identities are configured. No security bypass, driver load, forwarding/firewall
change, fresh Steam approval or console RUN occurred. Native start stays disabled.
Do not change the PS3 gateway or manually install the driver for this build.

Native mode currently requires an administrator app session; a separate UAC
broker/installer is still pending. Driver review, real Windows injection and
recovery validation, ordinary PSN traffic, and co-op acceptance remain required.
Windows 10 is untested. This remains a local development snapshot, not a tester
release. Bundle: dist/p2-middleware-0.1.2-dev-windows-x64/ with verified source.

Earlier 0.1.1 added the packet translator, binding and recovery model (57 tests).
Its original validation-network-20261025.json is retained unchanged.

## Initial desktop baseline

2026-10-25: initialized p2-middleware, Windows 11 x64 primary, Windows 10 x64
compatibility target (not yet tested).

Implemented: adapter discovery, validated TOML settings, in-memory Steam Guard
QR updates, cancel/forget/reauthorize, and externally routed bridge mode with a
private credential pipe, bounded lifetime and Windows Job Object cleanup.
Original source identities are recorded in source-import.json.

Pending: native forwarding/interception, elevated helper/driver, network-state
recovery, long-running operation, public tester package, Windows 10 and a new
live console RUN. Start Windows gateway is visibly disabled until implemented.

The prior router test achieved synchronized PS3-PC demo_viewer gameplay. It ended
with a PC crash of unknown cause; user defers investigation unless recurring.
That test does not establish acceptance of the new Windows packet path.

Validation: 33 workspace tests and 27 core tests without web-auth passed; formatting, Clippy with warnings denied and release build passed. The release GUI was visually inspected on Windows 11 build26300; read-only adapter discovery and gating matched expectations. The automated GUI startup/exit smoke returned 0 and left no desktop/bridge processes. See validation.json. No fresh
Steam approval or game session is claimed during project initialization.


A local development bundle with matching verified source is prepared by scripts/package.py under dist/p2-middleware-0.1.0-dev-windows-x64/. This is a local snapshot, not an off-device backup or public release.
