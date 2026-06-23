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
- `crates/runie-core/src/actors/config/tests.rs` (100ms sleep)
- `crates/runie-core/src/tests/login_logout/login_flow.rs` (10ms polling)
- `crates/runie-tui/src/effects/login.rs` (100ms sleep)
- `crates/runie-tui/src/ui_actor.rs` (50ms sleep)
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
- [ ] `status_timer_updates_over_time` and render tests still produce correct frames without real delays.

### Layer 4 — Provider Replay / E2E
- [ ] Provider validation timeout test still verifies timeout behavior without a 30-second leaked thread.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/tests.rs`
- `crates/runie-provider/src/tests.rs`
- `crates/runie-tui/src/tests/status_timer.rs`
- `crates/runie-core/src/actors/config/tests.rs`
- `crates/runie-core/src/tests/login_logout/login_flow.rs`
- `crates/runie-tui/src/effects/login.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/actor.rs`

## Implementation

### General patterns

Replace sleeps with one of:

1. `tokio::time::timeout(Duration, sub.recv()).await` for event-bus tests.
2. `tokio::time::pause()` + `advance()` for time-based tests.
3. `oneshot`/`watch` channels wired into the actor under test.

### 1. `fff_indexer/tests.rs`

Replace the initialization sleep with a `timeout`/`try_recv` loop that fails if the expected result never arrives:

```rust
let mut result = None;
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

Replace the 30-second background thread with a listener that never accepts:

```rust
#[tokio::test]
async fn test_validate_api_key_times_out_on_hanging_server() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        // Hold the connection open briefly; the validation timeout will fire first.
        std::thread::sleep(Duration::from_millis(500));
    });

    let start = std::time::Instant::now();
    let result = crate::validate_api_key_with_timeout(
        &format!("http://127.0.0.1:{}/v1", port),
        "sk-test",
        Duration::from_millis(250),
    )
    .await;

    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(2));
}
```

If the validation call returns before the listener thread wakes, that is fine; the listener thread exits quickly. The key change is reducing the background sleep from 30s to a value shorter than the test timeout.

### 3. `runie-tui/src/tests/status_timer.rs`

Replace `std::thread::sleep` with `tokio::time::pause` and `advance`:

```rust
#[test]
fn status_timer_updates_over_time() {
    tokio::time::pause();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.ensure_fresh();

    let out1 = render_status(&mut state);
    tokio::time::advance(Duration::from_millis(100)).await;
    state.ensure_fresh();
    let out2 = render_status(&mut state);

    assert!(out1.contains("Working"));
    assert!(out2.contains("Working"));
}
```

(If the test is a sync `#[test]`, make it `#[tokio::test]` or use `tokio::runtime::Runtime` to call `pause`/`advance`.)

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

Replace the polling loop with a single `timeout`/`recv` if possible, or keep the loop but yield without sleep:

```rust
let mut found = false;
for _ in 0..100 {
    if list_configured_providers().iter().any(|(n, _, _)| n == "minimax") {
        found = true;
        break;
    }
    tokio::task::yield_now().await;
}
assert!(found, "provider should be saved in the background");
```

### 6. `runie-tui/src/effects/login.rs`

The 100ms sleep is used to let a timeout fire. Replace with `tokio::time::pause` + `advance`:

```rust
tokio::time::pause();
run("openai".into(), "sk-test".into(), tx, provider_tx);
tokio::time::advance(Duration::from_millis(150)).await;

let event = collect_event(&mut rx).await;
assert!(matches!(event, CoreEvent::ValidationFailed { .. }));
```

### 7. `runie-tui/src/ui_actor.rs`

Same pattern: pause time and advance 60ms instead of sleeping 50ms.

### 8. `runie-agent/src/actor.rs`

Replace the 5ms polling with a `timeout`/`recv` loop:

```rust
let mut saw_error = false;
let mut saw_done = false;
for _ in 0..100 {
    if saw_error && saw_done {
        break;
    }
    tokio::task::yield_now().await;
    while let Some(Ok(evt)) = sub.try_recv() { ... }
}
```

If the test is timing-sensitive, increase iterations or use `tokio::time::timeout`.

### Step 9: Run tests

```bash
cargo test --workspace
```

### Step 10: Commit

```bash
git add crates/runie-core/src/actors/fff_indexer/tests.rs crates/runie-provider/src/tests.rs \
  crates/runie-tui/src/tests/status_timer.rs crates/runie-core/src/actors/config/tests.rs \
  crates/runie-core/src/tests/login_logout/login_flow.rs crates/runie-tui/src/effects/login.rs \
  crates/runie-tui/src/ui_actor.rs crates/runie-agent/src/actor.rs \
  tasks/remove-sleeps-from-automatic-tests.md tasks/index.json
git commit -m "test: remove sleep calls from automatic tests"
```

## Notes

- Add a CI grep check: `grep -R "sleep\|thread::sleep" crates/*/src/**/tests.rs` (or similar) to prevent regressions.
- Some tests may need minor actor changes to expose readiness signals; prefer small changes over large refactors.
