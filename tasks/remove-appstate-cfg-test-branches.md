# Remove `#[cfg(test)]` branches from `AppState` production methods

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/model/state/app_state.rs` contains `#[cfg(test)]` blocks inside production methods (`configured_providers`, `resolve_default_model`, `provider_config`, `remove_provider`, `set_provider_models`). These branches read `login_config` directly, diverging from production behavior. Tests should exercise the same code path as production.

## Acceptance Criteria

- [ ] No `#[cfg(test)]` blocks remain inside `AppState` production methods.
- [ ] Existing tests still pass by injecting config through the normal cache/channel path.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] Update tests that rely on the cfg(test) paths. Likely locations:
  - `crates/runie-core/src/commands/tests/model.rs`
  - `crates/runie-core/src/tests/login_logout/login_flow.rs`
  - `crates/runie-core/src/tests/login_logout/model_select_edge_cases.rs`
  - `crates/runie-core/src/login_config/tests.rs`
- Set `state.config_cache` directly or capture the sent `ConfigMsg` instead of relying on `login_config` side effects.

### Layer 2 — Event Handling
- [ ] If `remove_provider` / `set_provider_models` tests exist, verify the correct `ConfigMsg` is sent.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-core/src/commands/tests/model.rs`
- `crates/runie-core/src/tests/login_logout/*.rs`
- `crates/runie-core/src/login_config/tests.rs`

## Implementation

### Step 1: Replace `#[cfg(test)]` fallbacks with a consistent default

For `configured_providers` (lines 263–273):

```rust
pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
    self.config_cache
        .as_ref()
        .map(|c| c.configured_providers())
        .unwrap_or_default()
}
```

For `resolve_default_model` (lines 276–286):

```rust
pub fn resolve_default_model(&self) -> (String, String) {
    self.config_cache
        .as_ref()
        .map(|c| c.resolve_default_model())
        .unwrap_or_default()
}
```

For `provider_config` (lines 289–306):

```rust
pub fn provider_config(&self, name: &str) -> Option<crate::config::ModelProvider> {
    self.config_cache
        .as_ref()
        .and_then(|c| c.model_providers.get(name).cloned())
}
```

### Step 2: Remove test-only side effects from mutating methods

For `remove_provider` (lines 309–317):

```rust
pub fn remove_provider(&self, name: &str) {
    self.send_config_msg(crate::actors::ConfigMsg::RemoveProvider {
        name: name.to_string(),
    });
}
```

For `set_provider_models` (lines 320–332):

```rust
pub fn set_provider_models(&self, name: &str, models: Vec<String>) {
    self.send_config_msg(crate::actors::ConfigMsg::SetProviderModels {
        name: name.to_string(),
        models,
    });
}
```

### Step 3: Update tests

If tests need config present, set `state.config_cache` directly:

```rust
let mut state = AppState::default();
state.config_cache = Some(Config {
    provider: Some("openai".into()),
    models: ModelsConfig { default: Some("gpt-4o".into()), scoped: None },
    ..Config::default()
});
assert_eq!(state.resolve_default_model(), ("openai".into(), "gpt-4o".into()));
```

For tests that verify `remove_provider` sends a message, capture the message channel and assert the sent variant.

### Step 4: Run tests

```bash
cargo test -p runie-core app_state
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-core/src/model/state/app_state.rs \
  crates/runie-core/src/commands/tests/model.rs \
  crates/runie-core/src/tests/login_logout/login_flow.rs \
  crates/runie-core/src/tests/login_logout/model_select_edge_cases.rs \
  crates/runie-core/src/login_config/tests.rs \
  tasks/remove-appstate-cfg-test-branches.md tasks/index.json
git commit -m "refactor(core): remove cfg(test) branches from AppState"
```

## Notes

- If removing the branches breaks many tests, consider adding a test helper `AppState::with_test_config(...)` instead of restoring `cfg(test)`.
- The goal is one code path for production and tests.
