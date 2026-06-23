# Increase Tokio worker threads for runie-tui

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

`crates/runie-tui/src/main.rs` hard-coded `worker_threads = 2` for the multi-thread Tokio runtime. With UI, agent turn, provider actor, persistence, session store, and IO actors all sharing only two workers, CPU-heavy agent work (tool-call parsing, truncation, JSON repair) can starve the `UiActor` and delay input/animation updates.

## Acceptance Criteria

- [x] Remove `worker_threads = 2` from `#[tokio::main]` so the runtime uses the default number of cores.
- [x] `cargo check -p runie-tui` succeeds.
- [x] `cargo test -p runie-tui` succeeds.

## Tests

- [x] Layer 4 Smoke: `cargo test -p runie-tui` passes after the change.

## Files touched

- `crates/runie-tui/src/main.rs`

## Notes

Using the default core count keeps the event loop responsive without manually tuning a magic number.
