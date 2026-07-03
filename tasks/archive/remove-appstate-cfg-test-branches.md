# Remove AppState #[cfg(test)] branches and Option<Sender> handles

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`AppState` (in `model/state/app_state.rs`) carries 5 `Option<Sender>` actor handles (`config_tx`, `provider_tx`, `persistence_tx`, `session_store_tx`, `io_tx`) for test-vs-prod mode, plus a shared `approval_registry: Arc<Mutex<ApprovalRegistry>>`. Then `reset_session` (lines 172–196) carefully clones all of them across a reset, and five accessor methods (`configured_providers`, `resolve_default_model`, `provider_config`, `remove_provider`, `set_provider_models`) each carry a `#[cfg(test)]` direct-call branch that bypasses the actor system:

```rust
pub fn resolve_default_model(&self) -> (String, String) {
    if let Some(config) = self.config_cache.as_ref() {
        return config.resolve_default_model();
    }
    #[cfg(test)]
    { return crate::login_config::with_read_lock(|c| c.resolve_default_model()); }
    #[cfg(not(test))]
    (String::new(), String::new())
}
```

Under the actor / event-based posture this is a smell: state holds actor handles and branches on `cfg(test)` to skip them. The unification is either (a) always spawn lightweight actors in tests (so the `config_tx` path is exercised uniformly), or (b) route all config access through a single `ConfigState` cache that the `ConfigActor` keeps filled, dropping the `Option<Sender>` fields and the `#[cfg(test)]` branches.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Always-actor** — tests spawn a `ConfigActor` (with a temp config path) before constructing `AppState`; the `#[cfg(test)]` branches in `app_state.rs` deleted; the 5 accessors go through `config_tx` only; OR
  - (b) **Cache-only** — `ConfigActor` keeps `state.config_cache: Option<Config>` filled on every change; the 5 accessors read only from `config_cache` (no `cfg(test)` fallback); write operations (`remove_provider`, `set_provider_models`) fire-and-forget to `config_tx` without a `cfg(test)` synchronous mirror; the `Option<Sender>` fields stay for write routing but the read path is cache-only.
- [ ] `rg "#\[cfg\(test\)\]" crates/runie-core/src/model/state/app_state.rs` returns zero hits.
- [ ] `reset_session` no longer special-cases the 5 handles (or is simplified to a single `preserve_handles(&self) -> Self` helper).
- [ ] `cargo check --workspace` succeeds with no new warnings (including `--cfg test`).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `resolve_default_model_reads_from_cache` — with `config_cache` populated, `resolve_default_model()` returns the cached pair without any actor.
- [ ] `configured_providers_reads_from_cache` — with `config_cache` populated, `configured_providers()` returns the cached list.
- [ ] `provider_config_reads_from_cache` — `provider_config("openai")` returns the cached `ModelProvider`.
- [ ] `remove_provider_fires_config_msg` — `remove_provider("openai")` sends a `ConfigMsg::RemoveProvider` (asserted via a mock `mpsc::Sender`), no `cfg(test)` file write.

### Layer 2 — Event Handling
- [ ] `config_actor_keeps_cache_filled` — after `ConfigActor` processes a `ConfigLoaded` event, `state.config_cache` is `Some`.
- [ ] `reset_session_preserves_handles` — after `reset_session()`, the 5 actor handles still resolve (existing test stays green).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_test_suite_runs_without_cfg_test_branches` — `cargo test --workspace` passes with the `#[cfg(test)]` branches deleted.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` (delete `#[cfg(test)]` blocks, simplify accessors)
- `crates/runie-core/src/tests/` (test helpers that construct `AppState` without actors — switch to spawning a temp `ConfigActor` if option a)
- `crates/runie-core/src/update/config.rs` (ensure `ConfigLoaded` populates `config_cache`)

## Notes

Option (b) is lower-risk: it keeps the actor handles for write routing but makes the read path cache-only, which is what production already does. Option (a) is purer but requires every test helper (`fresh_state()` etc., already being deduped by `dedupe-fresh-state-test-helper`) to spawn a `ConfigActor`. The done task `centralize-app-state-ownership` already moved turn-queue ownership into `UiActor`; this task continues that line by removing the test-only bypasses. `remove-login-config-test-shim` (todo) overlaps — coordinate.
