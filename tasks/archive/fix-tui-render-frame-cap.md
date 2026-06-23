# Cap render-thread frame rate and coalesce snapshots

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

The dedicated render thread in `crates/runie-tui/src/main.rs` previously redrew the terminal for every snapshot it received. During streaming response bursts this burns CPU and can drop frames. It also called `terminal.size()` once per frame.

## Acceptance Criteria

- [x] Render loop waits at most one frame period (~16 ms) per iteration, capping redraws to ~60 FPS.
- [x] Pending snapshots are drained and only the latest one is drawn, coalescing intermediate frames.
- [x] `cargo check -p runie-tui` succeeds.
- [x] `cargo test -p runie-tui` succeeds.

## Tests

- [x] Layer 4 Smoke: `cargo test -p runie-tui` passes; render tests still draw snapshots correctly.

## Files touched

- `crates/runie-tui/src/main.rs`

## Notes

This trades a little latency for throughput: the worst case is one extra frame of delay, but the UI no longer chokes on burst traffic.
