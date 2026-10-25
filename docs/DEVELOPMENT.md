# Development

The root crate owns the desktop, settings, approval controller and worker lifetime.
`crates/bridge-core` owns protocol/encryption/login/lobby logic plus the p2-bridge
CLI. Both build from source in one workspace, without absolute research paths.

Rust 1.99.1, eframe 0.33.3 and Cargo.lock are pinned. The GUI uses OpenGL and native
accessibility support. Windows 10 compatibility is intentional but untested.
Use a local CARGO_TARGET_DIR for fast builds when the checkout is on a network share.

This desktop uses focused instructions rather than the generic server template's
DNS/extraction flows: no game material is required to build this middleware.

`--smoke-test` opens the normal UI and closes it after three seconds, without
Steam approval or a bridge listener. Tests cover inherited encrypted relay
contracts, configuration bounds, QR origins, pre-network cancellation, diagnostic
redaction and Windows Job Object cleanup. Live approval is a separate test.

`scripts/package.py` explicitly selects sources, verifies a source ZIP and creates
a local development bundle. It excludes runtime/game/capture/dump data. Dependency
metadata omits local cache paths. No public upload is performed.


Version 0.1.2 adds p2-route, the fixed windows_host.ps1 backend (embedded at build
time), protected journal storage and the GUI helper controller. p2-route status
is read-only. p2-route run fails before protected storage or system mutations
while APPROVED_DRIVER is None. Recovery requires an administrator session and
operates only on a validated owned journal. Package all three executables together.
The helper's reporting pipe is best-effort so GUI termination cannot abort cleanup.
Native driver and privileged host integration still need live acceptance.

Version 0.1.3 replaces helper stdin/stdout control with local named pipes using
Tokio overlapped I/O, explicit DACLs and mutual process-ID checks. ShellExecuteExW
runas launches only p2-route; it does not send settings or Steam tokens in command
arguments. `tests/helper_ipc.rs` launches the real helper without elevation for a
non-mutating Probe, stop/disconnect, invalid-message and driver-gate checks.
Those tests do not prove elevated setup, cross-integrity IPC or network recovery.
The GUI's Check helper button offers that read-only interactive UAC check.
