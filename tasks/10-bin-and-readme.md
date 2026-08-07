# Step 10: Binary entrypoint + README

**Status:** implemented; provider wiring remains intentionally adapter-owned
**Depends on:** 09

## Goal
A runnable `runie` binary + a short README documenting how to use it.

## Changes
- `crates/runie-tui/src/bin/runie.rs`:
  - `fn main() -> anyhow::Result<()>`:
    - Init tracing-subscriber.
    - Construct a `LoopActor` with a `MockStreamFn` or a future `StreamFn` adapter (placeholder for now; prints a TODO if no provider is configured).
    - Build the App, init crossterm raw mode + alternate screen, run to exit, restore terminal.
- `crates/runie-tui/README.md`: short doc — what `runie-tui` is, key bindings, how to run.

## Verification
- `cargo run -p runie-tui` → binary launches; in a real terminal, prompt is visible.
- `cargo build --release -p runie-tui` → exit 0.

## Notes
- Without a real `StreamFn` adapter, the binary is a UI shell. The README documents this as a placeholder; the StreamFn adapter is a separate task (not in this plan).
