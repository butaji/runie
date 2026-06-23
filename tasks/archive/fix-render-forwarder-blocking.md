# Fix render forwarder blocking

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

The render forwarder in `runie-tui/src/main.rs` used blocking `std::sync::mpsc::Sender::send()`, which blocked the async event loop when the render thread was busy. This caused input lag throughout the UI.

## Root Cause

`render_forwarder` used a regular `mpsc::channel()` with `send()`, which blocks until the receiver accepts the message. When the terminal render thread is busy, this blocked the entire async runtime.

## Fix

Changed to `std::sync::mpsc::SyncChannel(1)` with `try_send()`. If the render thread is backed up, frames are skipped instead of blocking:

```rust
fn render_forwarder(
    mut render_rx: watch::Receiver<Snapshot>,
    tx: std::sync::mpsc::SyncSender<Snapshot>,
) -> impl std::future::Future<Output = ()> {
    async move {
        loop {
            let snap = render_rx.borrow_and_update().clone();
            // Use try_send to avoid blocking the async event loop.
            if tx.try_send(snap).is_err() {
                // Render thread is backed up — skip this frame, let it catch up.
            }
            if render_rx.changed().await.is_err() {
                break;
            }
        }
    }
}
```

## Acceptance Criteria

- [x] `render_forwarder` uses `try_send()` instead of blocking `send()`
- [x] Uses `std::sync::mpsc::SyncChannel(1)` for bounded channel
- [x] `cargo test --workspace` succeeds
- [x] tmux integration tests pass

## Tests

### Layer 1 — State/Logic
- N/A (logic unchanged, only channel type changed)

### Layer 2 — Event Handling
- N/A (async channel behavior verified by integration tests)

### Layer 3 — Rendering
- N/A (render logic unchanged)

### Layer 4 — Smoke / E2E
- [x] tmux integration tests pass — verifies UI responsiveness

## Files touched

- `crates/runie-tui/src/main.rs`

## Notes

This fix resolves the "input delay" symptom reported by users. The root cause was a mismatch between async code and blocking channel operations.
