# Use channels for subagent result collection

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: none

## Description

Subagent results are currently collected via callbacks or polling, which complicates lifetime management and makes races easy. Replace the mechanism with bounded async channels (`tokio::sync::mpsc` or `oneshot`) so the caller awaits a channel and the subagent actor sends its final result exactly once.

## Acceptance Criteria

- [ ] Subagent actor sends results on a channel instead of invoking a callback.
- [ ] The caller awaits the channel with a timeout/cancellation path.
- [ ] No polling loops remain for subagent completion.
- [ ] Bounded channels provide backpressure; overflow is handled explicitly.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `subagent_channel_returns_result` — sender/receiver pair yields the expected result.
- [ ] `subagent_channel_drops_on_cancel` — cancellation closes the receiver cleanly.

### Layer 2 — Event Handling
- [ ] `subagent_event_sends_on_channel` — a subagent actor message produces a channel message.

### Layer 3 — Rendering
- [ ] N/A — subagent result collection is not TUI-specific.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `subagent_turn_awaits_channel` — a subagent run completes via the channel path in a provider replay test.

## Files touched

- `crates/runie-agent/src/subagent.rs`
- `crates/runie-agent/src/handle.rs`
- `crates/runie-agent/src/actor.rs`

## Notes

- Use `tokio::sync::oneshot` for a single final result; use `mpsc` if streaming intermediate events is required.
- Ensure the channel is dropped when the parent turn is cancelled to avoid leaking tasks.
- **Update after review:** `crates/runie-agent/src/subagent.rs` still uses `std::sync::Mutex<Vec<String>>` with a callback and polling; coordinate with `normalize-remaining-std-mutex-to-parking-lot.md`.
