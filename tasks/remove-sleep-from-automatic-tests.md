# Remove `sleep()` from automatic tests

**Status**: done
**Milestone**: R5
**Category**: Test harness
**Priority**: P2

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: none

## Description

`crates/runie-core/src/actors/session/tests.rs` and other test files used `tokio::time::sleep` for synchronization. AGENTS.md explicitly forbids artificial delays in automatic tests. Replaced all sleeps with deterministic `recv()` with timeout, which is a proper event-driven pattern.

## Changes Made

### Files Modified

1. **`crates/runie-core/src/actors/session/tests.rs`** - Rewrote all 5 tests to use `wait_for_event()` helper that uses `recv()` with timeout instead of polling with sleep.

2. **`crates/runie-core/src/actors/session/ractor_session_actor.rs`** - Updated tests module to use `wait_for_event()` helper, removing 2 sleep calls.

3. **`crates/runie-core/src/actors/config/ractor_config.rs`** - Updated `get_config_returns_config` test to wait for `ConfigLoaded` event deterministically instead of sleeping.

4. **`crates/runie-core/src/actors/input/actor.rs`** - Updated `insert_char_updates_cursor` test to wait for each `InputChanged` event deterministically.

5. **`crates/runie-core/src/actors/io/ractor_io.rs`** - Updated `ractor_io_receives_messages` test to use `recv()` with timeout.

6. **`crates/runie-core/src/actors/permission/ractor_permission.rs`** - Updated both tests to use `wait_for_event()` helper.

### Pattern Used

```rust
/// Wait for an event matching a predicate with a deterministic timeout.
async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
where
    F: Fn(&Event) -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let timeout_duration = deadline - tokio::time::Instant::now();
        match tokio::time::timeout(timeout_duration, sub.recv()).await {
            Ok(Ok(evt)) => {
                if pred(&evt) {
                    return true;
                }
            }
            Ok(Err(_)) | Err(_) => break,
        }
    }
    false
}
```

## Acceptance Criteria

- [x] Remove all `tokio::time::sleep` calls from `crates/runie-core/src/actors/session/tests.rs`.
- [x] Replace them with `tokio::sync::oneshot`/`notify` waits or pre-seeded state. (Used broadcast channel `recv()` with timeout - deterministic)
- [x] Verify no other automatic tests contain `sleep` (excluding harness polling deadlines, which should be documented).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [x] All session actor tests pass without delays.
- [x] All config actor tests pass without delays.
- [x] All input actor tests pass without delays.
- [x] All IO actor tests pass without delays.
- [x] All permission actor tests pass without delays.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Remaining Sleeps (Acceptable)

The following sleeps remain and are acceptable:

| File | Line | Reason |
|------|------|--------|
| `runie-testing/src/timeout.rs` | 63 | Tests the timeout utility itself; simulating long operation is correct |
| `runie-agent/src/tool_runner.rs` | 273 | Tests the timeout utility; simulating slow operation is correct |
| `runie-testing/src/runner.rs` | 98 | Harness polling loop (documented as acceptable) |
| `runie-provider/src/mock.rs` | 181, 297 | Mock provider simulating provider delays |
| `runie-provider/src/retry.rs` | 50 | Production retry logic |
| `runie-agent/src/tool_runner.rs` | 273 | Production timeout logic |

## Notes

- The `wait_for_event()` helper provides a deterministic 2-second deadline for event arrival.
- Tests no longer rely on arbitrary timing to observe async side effects.
- This makes tests more reliable and faster (no unnecessary waiting).
