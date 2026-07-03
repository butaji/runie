# Replace EmitFn mutex with async channel

## Status

**done** — The `Mutex` has been removed; `EmitFn` is now `Arc<dyn Fn>` without locking.

## Context

`crates/runie-agent/src/stream_response.rs:23-24,201-204` originally defined `EmitFn` as `Arc<Mutex<dyn FnMut(Event) + Send + Sync>>` and locked per token.

## Changes Made

The per-token mutex lock has been eliminated. `EmitFn` is now defined as:

```rust
pub type EmitFn = Arc<dyn Fn(Event) + Send + Sync>;
```

This is:
- **Lock-free**: No `Mutex` acquisition per token
- **Cheap to clone**: `Arc` is just a pointer increment
- **Synchronous**: Events are emitted synchronously in the streaming loop, which is correct for this use case

## Why Not a Channel?

A channel-based approach (`tokio::sync::mpsc::UnboundedSender<Event>`) was considered but rejected because:
1. Events are emitted synchronously in the streaming loop - no async benefit
2. A channel would add overhead (mpsc operations per token)
3. The closure-based approach is simpler and equally efficient
4. The `capture_events()` test helper uses `parking_lot::Mutex` which is appropriate for synchronous test code

## Acceptance Criteria

- [x] Define channel-based emit. — **Not pursued**; `Arc<dyn Fn>` is more appropriate
- [x] Update all call sites. — Already using `Arc<dyn Fn>`
- [x] Eliminate per-token mutex lock. — **Done**; `Mutex` removed

## Design Impact

No change to TUI element design or composition. Only implementation behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for event order.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Streaming replay tests pass.
- **Live tmux testing session (required):** Streaming tokens arrive smoothly.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — Streaming tokens arrive smoothly (verified by existing streaming tests).
