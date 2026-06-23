# Remove `sleep()` calls from automatic tests

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`AGENTS.md` forbids `sleep()` in automatic tests. Several tests still use `tokio::time::sleep` or `std::thread::sleep` to wait for asynchronous side effects. This task replaces them with deterministic synchronization.

Affected files:

- `crates/runie-core/src/actors/fff_indexer/tests.rs` (5s and 200ms sleeps)
- `crates/runie-provider/src/tests.rs` (30-second background sleep)
- `crates/runie-tui/src/tests/status_timer.rs` (100ms thread sleep)
- `crates/runie-core/src/actors/config/tests.rs` (100ms sleep in an `#[ignore]`d watcher test)
- `crates/runie-core/src/tests/login_logout/login_flow.rs` (10ms polling)
- `crates/runie-tui/src/ui_actor.rs` (50ms sleep before effect recv)
- `crates/runie-agent/src/actor.rs` (5ms polling)

## Acceptance Criteria

- [ ] No `sleep()` or `thread::sleep` remains inside `#[test]` or `#[tokio::test]` functions.
- [ ] All affected tests remain stable and fast.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- [ ] Each converted test still asserts the expected event/state.

### Layer 3 — Rendering
- [ ] `status_timer_updates_over_time` produces correct frames without real delays.

### Layer 4 — Provider Replay / E2E
- [ ] Provider validation timeout test still verifies timeout behavior without a 30-second leaked thread.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/tests.rs`
- `crates/runie-provider/src/tests.rs`
- `crates/runie-tui/src/tests/status_timer.rs`
- `crates/runie-core/src/actors/config/tests.rs`
- `crates/runie-core/src/tests/login_logout/login_flow.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/actor.rs`

## Implementation

### General patterns

Replace sleeps with one of:

1. `tokio::time::timeout(Duration, sub.recv()).await` for event-bus tests.
2. `tokio::task::yield_now().await` for short polling loops.
3. Manually constructing older `Instant` values for timer display tests.

### 1. `fff_indexer/tests.rs`

Replace the initialization sleep with a `timeout`/`try_recv` loop that fails if the expected result never arrives:

```rust
let mut result = None;
let mut sub = bus.subscribe();
for _ in 0..50 {
    if let Some(Ok(FffSearchResult(payload))) = sub.try_recv() {
        if payload.request_id == request_id {
            result = Some(payload);
            break;
        }
    }
    tokio::task::yield_now().await;
}
let result = result.expect("got a result for request_id");
```

Remove the explicit `tokio::time::sleep(Duration::from_secs(5))` calls. If the indexer needs more time in CI, increase the loop iterations, not the sleep.

### 2. `runie-provider/src/tests.rs`

Replace the 30-second background sleep with a shorter hold so the thread exits quickly:

```rust
std::thread::spawn(move || {
    let (_stream, _) = listener.accept().unwrap();
    // Hold the connection open briefly; the validation timeout will fire first.
    std::thread::sleep(Duration::from_millis(500));
});
```

### 3. `runie-tui/src/tests/status_timer.rs`

Avoid real time progression by setting `turn_started_at` to a past `Instant`:

```rust
#[test]
fn status_timer_updates_over_time() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
    state.ensure_fresh();

    let out = render_status(&mut state);
    assert!(out.contains("Working"));
    assert!(out.contains("2.0s") || out.contains("2s"), "timer should reflect elapsed time");
}
```

Remove `std::thread::sleep` entirely.

### 4. `runie-core/src/actors/config/tests.rs`

Replace the 100ms sleep with a deterministic wait for the watcher event:

```rust
std::fs::write(&path, r#"provider = "anthropic""#).unwrap();
let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
    .await
    .unwrap()
    .unwrap();
assert!(matches!(event, Event::ConfigLoaded { .. }));
```

### 5. `runie-core/src/tests/login_logout/login_flow.rs`

Replace the polling loop with yielding:

```rust
for _ in 0..50 {
    if list_configured_providers().iter().any(|(n, _, _)| n == "minimax") {
        break;
    }
    tokio::task::yield_now().await;
}
assert!(
    list_configured_providers().iter().any(|(n, _, _)| n == "minimax"),
    "provider should be saved in the background"
);
```

### 6. `runie-tui/src/ui_actor.rs`

Remove the 50ms sleep before the timeout and rely on the timeout itself:

```rust
actor.handle_event(Event::Submit, effect_tx.clone()).await;

let result = tokio::time::timeout(Duration::from_secs(2), effect_rx.recv()).await;
assert!(
    result.is_ok(),
    "validation effect should produce a result event"
);
```

### 7. `runie-agent/src/actor.rs`

Replace the 5ms polling with yielding:

```rust
let mut saw_error = false;
let mut saw_done = false;
for _ in 0..100 {
    if saw_error && saw_done {
        break;
    }
    tokio::task::yield_now().await;
    while let Some(Ok(evt)) = sub.try_recv() {
        match evt {
            Event::Error { .. } => saw_error = true,
            Event::Done { .. } => saw_done = true,
            _ => {}
        }
    }
}
```

### Step 8: Run tests

```bash
cargo test --workspace
```

### Step 9: Commit

```bash
git add crates/runie-core/src/actors/fff_indexer/tests.rs crates/runie-provider/src/tests.rs \
  crates/runie-tui/src/tests/status_timer.rs crates/runie-core/src/actors/config/tests.rs \
  crates/runie-core/src/tests/login_logout/login_flow.rs crates/runie-tui/src/ui_actor.rs \
  crates/runie-agent/src/actor.rs tasks/remove-sleeps-from-automatic-tests.md tasks/index.json
git commit -m "test: remove sleep calls from automatic tests"
```

## Notes

- Add a CI grep check to prevent regressions: `grep -R "sleep(" crates/*/src/**/tests.rs crates/*/src/**/tests/**/*.rs`.
- Some tests may need minor actor changes to expose readiness signals; prefer small changes over large refactors.
