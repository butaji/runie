# Step 01: Scaffold runie-tui crate

**Status:** pending
**Depends on:** none

## Goal
Add a new `runie-tui` crate to the workspace with dependencies and an empty
src tree.

## Changes
- `crates/runie-tui/Cargo.toml`: package metadata; deps `runie-core` (path),
  `ratatui = "0.29"`, `crossterm = "0.28"`, `tui-textarea = "0.7"`,
  `tokio` (full features), `tracing`, `tracing-subscriber`, `anyhow`,
  `futures`. `[[bin]]` entry for `runie`.
- `crates/runie-tui/src/lib.rs`: re-export stubs.
- `crates/runie-tui/src/bin/runie.rs`: empty `fn main()`.
- Add `crates/runie-tui` to workspace members in root `Cargo.toml`.

## Verification
- `cargo check -p runie-tui` → exit 0.
- `ls crates/runie-tui/src` shows the layout skeleton.

## Notes
- `ratatui 0.29` matches grok-build's pin. If our rustc complains, fall back to 0.30.
- Workspace member list edit goes in `Cargo.toml` next to the existing `crates/runie-core` line.