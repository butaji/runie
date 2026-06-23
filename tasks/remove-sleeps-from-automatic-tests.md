# Remove remaining `sleep()` calls from automatic tests

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`AGENTS.md` forbids `sleep()` in automatic tests. Most sleeps have already been removed; only two sources remain:

- `crates/runie-core/src/actors/fff_indexer/tests.rs` — three `tokio::time::sleep(Duration::from_millis(200)).await` calls while waiting for the indexer actor to initialize.
- `crates/runie-provider/src/tests.rs` — one `std::thread::sleep(Duration::from_millis(500))` in the hanging-server timeout test.

This task replaces them with deterministic synchronization.

## Acceptance Criteria

- [ ] No `sleep()` or `thread::sleep` remains in `fff_indexer/tests.rs` or `runie-provider/src/tests.rs`.
- [ ] All affected tests remain stable and fast.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- [ ] Each converted test still asserts the expected event/state.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- [ ] Provider validation timeout test still verifies timeout behavior without a long leaked thread.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/tests.rs`
- `crates/runie-provider/src/tests.rs`

## Implementation

### 1. `fff_indexer/tests.rs`

Replace each `tokio::time::sleep(Duration::from_millis(200)).await` with a deterministic wait for the actor to be ready.

Option A: If the actor exposes a readiness signal, await it.

```rust
// If FffIndexerActor provides a ready() future:
tokio::time::timeout(Duration::from_secs(5), indexer.ready()).await.unwrap();
```

Option B: Poll with `yield_now` and a bounded retry count:

```rust
for _ in 0..200 {
    if indexer.is_ready() {
        break;
    }
    tokio::task::yield_now().await;
}
assert!(indexer.is_ready(), "indexer should initialize");
```

Option C: Use `tokio::time::pause` + `advance` if the actor uses Tokio timers internally.

Choose the option that matches the actor's internals.

### 2. `runie-provider/src/tests.rs`

Replace the 500ms background sleep with a listener that never accepts (so the OS times out the connection) or a much shorter hold:

```rust
std::thread::spawn(move || {
    let (_stream, _) = listener.accept().unwrap();
    std::thread::sleep(Duration::from_millis(50));
});
```

Better: avoid spawning a long-lived thread entirely by using a `tokio::net::TcpListener` that is never accepted, and let the validation call hit its own timeout:

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let port = listener.local_addr().unwrap().port();

let handle = tokio::spawn(async move {
    let _ = listener.accept().await; // never completes
});

let start = std::time::Instant::now();
let result = validate_api_key_with_timeout(
    &format!("http://127.0.0.1:{}/v1", port),
    "sk-test",
    Duration::from_millis(250),
).await;

handle.abort();
assert!(result.is_err());
assert!(start.elapsed() < Duration::from_secs(2));
```

### Step 3: Run tests

```bash
cargo test -p runie-core fff_indexer
cargo test -p runie-provider validate_api_key
cargo test --workspace
```

### Step 4: Commit

```bash
git add crates/runie-core/src/actors/fff_indexer/tests.rs crates/runie-provider/src/tests.rs \
  tasks/remove-sleeps-from-automatic-tests.md tasks/index.json
git commit -m "test: remove remaining sleep calls from automatic tests"
```

## Notes

- Add a CI grep check to prevent regressions: `grep -R "sleep(" crates/*/src/**/tests.rs crates/*/src/**/tests/**/*.rs`.
