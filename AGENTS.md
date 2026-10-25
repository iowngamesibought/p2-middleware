# P2 middleware development

Read README.md and docs/CURRENT.md first, then only directly needed files.
Keep the user-selected name p2-middleware and this directory.

- Windows 11 x64 is primary; Windows 10 x64 remains untested until explicitly run.
- This is a local bridge to official services, not a complete private backend.
- Maintain owned Rust sources and upstream attribution. Import source, not old binaries.
- No game files, raw tickets, captures, memory dumps or account credentials in the publishable tree.
- Tokens remain in memory; never put them in settings, logs, arguments or backups.
- Cancellation must discard late approval results. Closing the app must stop its owned worker.
- Do not claim native Windows gateway readiness from the earlier router RUN.
- Keep network changes scoped and reversible. Never disable the system firewall or replace unrelated router rules.
- Update docs/CURRENT.md with actual tests and limits. Snapshot source, lockfiles and notices without runtime files.
- Preserve existing licensing; do not invent an owner-selected project license or claim public release readiness.

