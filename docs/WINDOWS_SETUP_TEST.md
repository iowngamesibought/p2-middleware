# Historical setup diagnostic (0.1.5–0.1.7)

The 0.1.8 GUI replaces these manual steps with automatic startup/close cleanup
and one Confirm PS3 action. The text below is historical evidence, not current
user instructions. See README.md for the current flow.

# Windows setup and restoration test

Version 0.1.5 adds a bounded mutating diagnostic so the Windows host operations
can be validated separately from the unresolved packet-driver dependency.

## Run the test

Open p2-middleware.exe normally from the latest bundle (0.1.7; recover any previous failed transaction first). Select the detected LAN
adapter and enter the PS3 address (10.0.0.220 in the existing test setup, if
unchanged). The PC address, router and prefix must match the detected adapter.
Do not replace the detected router with a guess from the console's old settings.
No running PS3 or Steam sign-in is required; keep the console gateway unchanged.

Expand **Test Windows setup and restoration** and choose **Run setup and restore
test (Windows permission)**. Approve the Windows prompt for p2-route. This is not
the earlier read-only helper test: it briefly changes selected-adapter forwarding
and creates the scoped firewall rule before restoring the recorded state.

Success is **Windows setup and restoration test passed. Keep the PS3 gateway
unchanged.** It proves only the applied/read-back/restored Windows settings in
this run. It does not prove driver loading, interception, PSN connectivity or co-op.

If it fails, read Activity. Interface or firewall-read errors occur before a new
routing transaction. Failure during mutation triggers recovery. If recovery is
incomplete, the journal remains and blocks new sessions; keep the package in its
original location and use **Recover previous network settings** after reviewing
adapter/rule identity. Do not retry repeatedly or delete the protected journal.

## Exact changes and boundaries

The helper first requires verified local control, administrator status, valid
settings, matching live adapter/address/router/prefix, no existing recovery record
and no conflicting owned rule. It uses the same protected ProgramData journal
and exclusive ownership as native mode. The protected directory and owner.lock
may remain after success; the routing record and owned firewall rule should not.

The rule allows inbound TCP27017 only from the configured PS3 address, to the PC
address, on the selected interface, for the sibling p2-bridge.exe. No listener is
opened. IPv4 forwarding changes only on that adapter in ActiveStore. If it was
already enabled, the test leaves it enabled. It does not change DHCP, addresses,
DNS, routes, the router, Windows protection or the system firewall's enabled state.

There is no deliberate hold period. After setup it checks the installed rule and
forwarding state, then restores immediately. Individual Windows commands have
20-second timeouts; the full test can take longer than 20 seconds. Stop or GUI
closure does not kill the helper during this transaction; its cleanup continues.
A helper crash or power failure may require explicit journal recovery.

Recovery reads back actual state before clearing the record, including commands
that returned success but failed to make their intended change. Changed adapter
identity or modified rules cause a retained record rather than removing unrelated
state. Concurrent administrator changes and delayed OS operations are still live
validation risks; do not run other network-management tools during the test.

## Implementation and validation

The dedicated CheckSetup request calls routing::check_setup. It shares transaction
and recovery code with native activation, but does not call driver_preflight or
start_capture. It does not approve driver identities or unlock Start Windows
gateway. It requires an explicit user action and administrator approval.

Automated tests use a fake host to verify success, preexisting forwarding,
post-mutation errors and cleanup readback failures. The real non-admin helper
rejects CheckSetup before opening protected storage or changing networking.
Tests never request elevation or exercise host mutations. Actual elevated Windows
acceptance remains pending the user's run; no game RUN is inferred.

## First live attempt and fix

The user's 0.1.5 attempt failed while looking up the new rule name, before any
firewall or forwarding mutation. The read-only lookup itself succeeded, but
PowerShell serialized the absent result to empty stdout; Rust expected JSON.
Version 0.1.6 emits explicit null and passes the real missing-rule regression.
The subsequent v0.1.6 retry reached mutation but failed during ownership readback;
see the recovery instructions below. The full elevated transaction is not yet accepted.

## Second live attempt: recovery before retry

Version 0.1.7 normalizes scalar firewall address/interface fields before indexing.
The v0.1.6 attempt left one scoped rule while forwarding was observed Disabled.
Its saved transaction must be recovered before another setup test. Journal schema
1 expects the original bridge executable location: moving to a new bundle folder
before recovery can cause a legitimate ownership conflict.

For the current local repair, preserve the original v0.1.6 helper and manifest,
replace only that folder's helper with the source-built v0.1.7 helper, and record
the hashes in a repair receipt. The original manifest deliberately remains original
and no longer describes the replaced helper. The separate v0.1.7 bundle remains
self-contained and unchanged. In the currently open v0.1.6 app, choose **Recover
previous network settings** and approve Windows. Wait for completed recovery;
verify the old rule is absent and forwarding remains Disabled. Do not delete the
protected journal or claim success from the read-only query alone.
