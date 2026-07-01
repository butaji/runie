# Remove runtime presence branching in submit user message

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: fix-tui-turn-complete-leaves-working-status-and-queued

## Description

`submit_user_message` and `run_bash_command` branched on `tokio::runtime::Handle::try_current()` to decide whether to send an actor message or mutate state directly. This was inconsistent and confusing.

## Fix Applied

Removed `tokio::runtime::Handle::try_current()` checks. Now the code only checks `state.actor_handles().is_some()`:

```rust
// Before: checked both handles AND runtime presence
if let Some(ref h) = handles {
    if tokio::runtime::Handle::try_current().is_ok() {
        // Production mode: send to TurnActor
        let _ = h.turn.try_send(TurnMsg::SubmitUserMessage { content, id });
    } else {
        // Test mode: apply synchronously
        self.apply_user_message_sync(content);
    }
}

// After: only check handles
if let Some(ref h) = handles {
    // Production mode: send to TurnActor
    let _ = h.turn.try_send(TurnMsg::SubmitUserMessage { content, id });
} else {
    // Test mode without handles: apply synchronously
    self.apply_user_message_sync(content);
}
```

Same pattern applied to `run_bash_command`.

## Acceptance Criteria

- [x] The code branches only on `state.actor_handles().is_some()`.
- [x] Tests that do not spawn actors take the synchronous/state-only path.
- [x] Tests with actor handles take the production path.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux message submission still works.

## Files touched

- `crates/runie-core/src/update/input/submit.rs`

## Validation

- `cargo test --workspace`: 733 passed, 0 failed
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
