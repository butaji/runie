# Fix `HeadlessRuntime` spin-waiting for config load

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/headless_runtime.rs:46-57` busy-loops with `try_recv` and `tokio::task::yield_now()` while waiting for `ConfigLoaded` or `Error`. It also discards the timeout result. The fix is to await the first matching event with a timeout and return a clear error if config never loads.

## Acceptance Criteria

- [ ] `HeadlessRuntime::spawn` no longer spin-waits.
- [ ] A timeout waiting for config produces a clear error instead of a runtime with no config.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- [ ] Add `headless_runtime_returns_error_when_config_never_loads` in `crates/runie-core/src/headless_runtime.rs` tests:
  - Spawn `HeadlessRuntime` with a stub `ProviderFactory`.
  - Do not emit `ConfigLoaded`.
  - Assert `spawn` returns an error or a runtime whose `config().await` is `None` within a short timeout.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/headless_runtime.rs`

## Implementation

### Step 1: Refactor `spawn` to await an event

Change the signature to fallible:

```rust
pub async fn spawn(
    bus: EventBus<Event>,
    factory: Arc<dyn ProviderFactory>,
) -> Result<Self, anyhow::Error> {
    let mut sub = bus.subscribe();
    let (config_handle, config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, provider_actor) =
        ProviderActor::spawn(bus, config_handle.clone(), factory);

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub.recv().await {
                Ok(Event::ConfigLoaded { .. }) => break Ok(()),
                Ok(Event::Error { message }) => break Err(anyhow::anyhow!(message)),
                _ => continue,
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for config to load"))??;

    Ok(Self {
        config_handle,
        provider_handle,
        _config_actor: config_actor,
        _provider_actor: provider_actor,
    })
}
```

### Step 2: Update callers

Search for `HeadlessRuntime::spawn(...).await`:

```bash
grep -R "HeadlessRuntime::spawn" crates/
```

Update each call site to handle the new `Result`:

```rust
let rt = HeadlessRuntime::spawn(bus, factory)
    .await
    .expect("config must load");
```

If callers currently expect `Self`, either propagate the `Result` or map it to a panic with context.

### Step 3: Add test

```rust
#[tokio::test]
async fn headless_runtime_errors_when_config_actor_stalls() {
    let bus = EventBus::<Event>::new(4);
    let factory = Arc::new(FailingProviderFactory);
    // Do not let ConfigActor load by pointing it at an invalid path? Simpler:
    // spawn and expect timeout because no ConfigLoaded is emitted.
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        HeadlessRuntime::spawn(bus, factory),
    )
    .await;
    assert!(result.is_err() || result.unwrap().is_err());
}
```

(Use a real stub factory; the test only cares about the timeout path.)

### Step 4: Run tests

```bash
cargo test -p runie-core headless_runtime
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-core/src/headless_runtime.rs tasks/fix-headless-runtime-spinwait.md tasks/index.json
git commit -m "fix(core): await config load in HeadlessRuntime instead of spin-waiting"
```

## Notes

- `sub.recv()` is async; using it removes the need for `try_recv`/`yield_now`.
- If changing the return type is too invasive, keep `Self` and panic on timeout with a clear message, but returning `Result` is preferred.
