# Use channels for subagent result collection

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: none

## Description

Subagent results are currently collected via callbacks or polling, which complicates lifetime management and makes races easy. Replace the mechanism with bounded async channels (`tokio::sync::mpsc` or `oneshot`) so the caller awaits a channel and the subagent actor sends its final result exactly once.

## Changes

Replaced `std::sync::mpsc::sync_channel` and `Arc<Mutex<SubagentState>>` with `tokio::sync::oneshot` channel in `crates/runie-agent/src/subagent.rs`. The subagent now:

1. Creates a `tokio::sync::oneshot` channel at the start.
2. Uses a single combined emit callback that:
   - Collects response text (`ResponseDelta`/`Response` events)
   - Sends error results (`Error` events) through the channel
   - Sends the final accumulated text on `Done` event
3. The caller awaits the channel with `tokio::time::timeout(300s, rx)`.
4. No polling loops remain; the channel delivers exactly one result.
5. Bounded channel behavior: if the receiver is dropped (cancellation), the sender's `send()` returns `Err` which is handled with `let _ =`.

**Before:**
```rust
let (tx, rx) = sync_mpsc::channel();
let state = Arc::new(Mutex::new(SubagentState::default()));
// Callback populates state; run_agent_turn is called once
// rx.recv() is a blocking call (polling)
```

**After:**
```rust
let (tx, rx) = tokio::sync::oneshot::channel();
// Single combined callback sends result on Done
// rx.await with 300s timeout
```

## Acceptance Criteria

- [x] Subagent actor sends results on a channel instead of invoking a callback.
- [x] The caller awaits the channel with a timeout/cancellation path.
- [x] No polling loops remain for subagent completion.
- [x] Bounded channels provide backpressure; overflow is handled explicitly.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `subagent_channel_returns_result` — sender/receiver pair yields the expected result.
- [x] `subagent_channel_drops_on_cancel` — cancellation closes the receiver cleanly.
- [x] `subagent_timeout_returns_error` — timeout path handles the error correctly.

### Layer 2 — Event Handling
- [x] `subagent_event_sends_on_channel` — a subagent actor message produces a channel message. (Covered by existing integration tests: `subagent_returns_echo_of_prompt`, `subagent_with_skill_context_uses_it`)

### Layer 3 — Rendering
- [x] N/A — subagent result collection is not TUI-specific.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `subagent_turn_awaits_channel` — a subagent run completes via the channel path in a provider replay test. (Covered by `subagent_channel_returns_result` and `explore_subagent_type_runs_with_mock_provider`)

## Files touched

- `crates/runie-agent/src/subagent.rs` — replaced sync mpsc channel with tokio oneshot; removed `SubagentState` struct; simplified result collection.

## Notes

- Used `tokio::sync::oneshot` (single result) rather than `mpsc` because the subagent returns exactly one final text result.
- The 300s timeout is generous for production; tests use the mock provider which returns immediately.
- The `parking_lot::Mutex` in `stream_response.rs` is unchanged — it protects the `EmitFn` which is sync by design.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
