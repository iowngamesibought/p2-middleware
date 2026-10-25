# Current helper lifecycle: 0.1.8

Normal startup sends Session after mutual process verification and UAC. The helper
holds the exclusive protected-store lock, restores any saved transaction, and
reports session_clean. Confirm sends Start only after bridge readiness; the driver
gate remains enforced before mutations. Reset restores and returns to idle.
Stop, parent EOF or malformed input restores and exits. Normal GUI close waits
for this result. Session idle has no expiry; active bridge/routing retains the
10-minute bound. Network errors stay visible; no generic completion masks them.
Steam credentials never enter this protocol. The GUI and account sign-in remain
unelevated. Startup/close cleanup needs no driver. A cleanup failure prevents new
configuration and preserves its record for the next startup/Retry.

Schema2 journals store the exact bridge executable identity for firewall matching.
Legacy schema1 migration obtains that identity from only the journal-named rule,
requires an absolute p2-bridge.exe path, and validates the full ownership shape
and both journal IPs. It never executes or loads the recovered path.

Simulated lifecycle tests cover startup leftovers, multiple confirmations separated
by Reset, failed setup, failed cleanup, malformed commands and parent disconnect.
Real elevated execution of this lifecycle remains pending. The earlier commands
below are retained for developer compatibility, not exposed as user setup steps.

## Historical control implementation: 0.1.3

The desktop and Steam approval run with the user's normal permissions. Explicit
network setup/recovery actions start only p2-route through ShellExecuteExW's runas
verb. A background worker handles the Windows approval prompt so the GUI remains
responsive. The driver gate is checked before offering elevation for native start.
This is an unsigned development package, not an installed Windows service.

## Control boundary

The GUI creates one local named pipe before launch, with a cryptographically
random 128-bit suffix. First-instance ownership prevents silently attaching to
an existing server; remote clients are rejected. Its explicit protected DACL
allows the desktop user, SYSTEM and Administrators. Handles are not inherited.
Tokio provides overlapped I/O rather than blocking the GUI or polling pipe flags.

The GUI compares the connecting process ID against the actual process handle
returned by ShellExecuteExW. The helper compares the server PID with its expected
parent. No configuration is sent until this check succeeds. Client pipe security
uses identification-only impersonation rights, so an unprivileged pipe server
cannot impersonate the administrator helper.

Only the fixed pipe name and parent PID appear in helper arguments. The initial
message is Start(validated network settings), CheckSetup(validated network settings),
Recover, or Probe. CheckSetup is the explicit mutating diagnostic documented in
WINDOWS_SETUP_TEST.md; it does not capture traffic. Unknown fields
and commands fail closed. No Steam credential or PSN ticket is part of this
protocol. Messages are capped at 16 KiB. Partial frames survive read timeouts;
truncated frames and oversized input fail. A 30-second initial-request timeout
prevents an idle helper from waiting indefinitely.

PowerShell operations remain fixed embedded source with JSON stdin. The helper
resolves PowerShell from the Windows system directory, sets a system-only module
search path and disables profiles. It does not execute caller-provided commands,
script paths, firewall names or executable paths.

## Stop and recovery

The GUI can request stop without killing the helper. The helper treats any extra
control message, EOF or read error as a stop request once activation completes.
It restores recorded changes on its own; closed event output cannot prevent that.
A GUI process exit closes the pipe. A helper process crash still needs journal
recovery. The helper remains outside the bridge's kill-on-close Job Object.

If the user stops while a Windows permission prompt is outstanding, the app
discards a subsequent approval before sending any setup command. It does not
try to approve or dismiss the Windows security prompt automatically. The helper
will receive EOF and exit. User denial is reported separately from launch errors.

Activation includes synchronous bounded Windows commands. Cancellation during
activation is handled when those commands return, then restoration runs. The
controller retains the connection for up to 120 seconds after a stop request;
timed-out sessions close the connection and report an error without force-killing
the helper. Retained journals must be inspected/recovered before a new session.

## Read-only helper check

Open the desktop normally and choose **Check helper (Windows permission)**.
Windows requests approval to launch p2-route. The Probe command reports the
helper's actual token elevation status and exits after five seconds or stop.
It never creates recovery storage, loads a driver or changes network settings.
It requires neither Steam sign-in nor a PS3. After the initial build validation,
the user ran this check and the Activity panel showed three successful elevated
helper results. See validation-helper-user-check-20261025.json. UAC interaction
itself and the GUI token level were not independently observed; cancellation and
alternate-account flows remain untested.

## Validation boundaries

Real non-admin subprocess tests cover probe events, stop, parent disconnection,
unknown commands/fields and refusal of an unapproved native start. Native pipe
tests cover exclusive endpoint ownership, actual client PID rejection and bounded
framing through cancelled reads. A read-only system-shell round trip validates
the embedded script invocation. These do not prove cross-integrity transport,
alternate administrator credentials, approved/cancelled UAC UI behavior, driver
loading, privileged ACL creation, actual network changes or gameplay.

Protected installation, helper signing and package provenance checks remain
deployment work. Keep all three executables together and retain the original
package location until any network recovery succeeds. Native gateway start stays
disabled until driver review and real network acceptance are complete.

API references:

- https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecuteexw
- https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-getnamedpipeclientprocessid
- https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid
- https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew
- https://docs.rs/tokio/1.54.1/tokio/net/windows/named_pipe/index.html
