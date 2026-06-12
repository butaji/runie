# Non-interactive modes as separate binaries

`runie-print` and `runie-json` are separate binaries that use `runie-core` and `runie-agent` directly, bypassing the actor system and TUI entirely.

`runie` (interactive) uses the full actor bus with UIAgent and terminal rendering.

Sharing logic via crates, not sharing runtime infrastructure.
