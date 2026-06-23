# Fix ConfigActor swallowing config write errors

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

All mutating helpers in `crates/runie-core/src/actors/config/actor.rs` (`save_provider`, `remove_provider`, `set_default_model`, `set_provider_models`) check `result.is_ok()` on the `JoinHandle` result. That only verifies the blocking task did not panic; it ignores whether the actual file operation succeeded. Failed writes therefore silently reload stale config. The fix is to match the inner `Result` and emit an error event on failure.

## Acceptance Criteria

- [ ] Config write failures surface as `Event::Error` instead of a silent `ConfigLoaded`.
- [ ] Successful writes still reload and emit `ConfigLoaded`.
- [ ] Layer 2 tests verify both success and failure paths.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A — behavior is actor/event wiring.

### Layer 2 — Event Handling
- [ ] Add `config_actor_emits_error_on_failed_save` in `crates/runie-core/src/actors/config/tests.rs`:
  - Spawn `ConfigActor` pointing at a read-only path (e.g., a file inside a read-only directory).
  - Send `ConfigMsg::SaveProvider`.
  - Collect events and assert an `Event::Error { .. }` is received and no second `ConfigLoaded` appears after the initial one.
- [ ] Keep existing `config_actor_loads_and_emits` test green.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs` — update mutating helpers.
- `crates/runie-core/src/actors/config/tests.rs` — add failure test.

## Implementation

### Step 1: Introduce a shared result handler

Add a private method to `ConfigActor`:

```rust
impl ConfigActor {
    async fn handle_write_result(
        &mut self,
        result: Result<anyhow::Result<()>, tokio::task::JoinError>,
        bus: &EventBus<Event>,
    ) {
        match result {
            Ok(Ok(())) => self.load_and_emit(bus).await,
            Ok(Err(e)) => {
                tracing::error!("config write failed: {e:?}");
                bus.publish(Event::Error {
                    id: "config".to_string(),
                    message: format!("Config write failed: {e}"),
                });
            }
            Err(e) => {
                tracing::error!("config write task panicked: {e:?}");
                bus.publish(Event::Error {
                    id: "config".to_string(),
                    message: format!("Config write task panicked: {e}"),
                });
            }
        }
    }
}
```

### Step 2: Replace `if result.is_ok()` in each mutating helper

For `save_provider`:

```rust
let result = tokio::task::spawn_blocking(move || {
    save_provider_to_path(&path, &name, &base_url, &api_key, &models)
})
.await;
self.handle_write_result(result, bus).await;
```

Do the same for `remove_provider`, `set_default_model`, and `set_provider_models`.

### Step 3: Add failure test

Create a read-only temp directory for the test:

```rust
#[tokio::test]
async fn config_actor_emits_error_on_failed_save() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    // Make parent read-only after creating the actor so initial load succeeds.
    let readonly = tmp.path().to_path_buf();
    let perms = std::fs::metadata(&readonly).unwrap().permissions();
    let mut readonly_perms = perms.clone();
    readonly_perms.set_readonly(true);

    let bus = EventBus::<Event>::new(8);
    let mut sub = bus.subscribe();
    let (handle, _actor) = ConfigActor::spawn(bus.clone(), Some(path.clone()));

    // Drain initial ConfigLoaded.
    let _ = tokio::time::timeout(Duration::from_secs(2), sub.recv()).await;

    std::fs::set_permissions(&readonly, readonly_perms).unwrap();
    handle
        .save_provider("openai", "https://api.openai.com/v1", "sk-test", vec!["gpt-4o".into()])
        .await;

    let mut saw_error = false;
    for _ in 0..20 {
        if let Ok(Some(Ok(Event::Error { .. }))) =
            tokio::time::timeout(Duration::from_millis(50), sub.recv()).await
        {
            saw_error = true;
            break;
        }
    }

    std::fs::set_permissions(&readonly, perms).unwrap();
    assert!(saw_error, "expected Event::Error after failed write");
}
```

(Adjust for the actual `ConfigActorHandle` API; if it does not expose `save_provider`, use the message channel directly.)

### Step 4: Run tests

```bash
cargo test -p runie-core config_actor_emits_error_on_failed_save
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-core/src/actors/config/actor.rs crates/runie-core/src/actors/config/tests.rs tasks/fix-config-actor-write-errors.md tasks/index.json
git commit -m "fix(core): propagate config write errors from ConfigActor"
```

## Notes

- Ensure the `Event::Error` variant has a `message` field. If it does not, use the correct variant.
- If tests run as root on CI, read-only directory tricks may not fail; use an invalid path (e.g., path containing a nonexistent parent) instead.
