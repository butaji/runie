# Fix /model and /provider slash commands report no providers configured

**Status**: todo
**Milestone**: R7
**Category**: Configuration
**Priority**: P2

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

In the live TUI with the mock provider configured and working, `/model` and `/provider` report `No connected providers. Use /provider to add a provider first.` The settings dialog (`/settings`) correctly shows `Provider: mock` and `Model: echo`, so the configuration is present but `configured_providers()` does not include the mock provider.

## Live Evidence

```
  No connected providers. Use /provider to add a provider first.
```

`/settings` output:
```
  Provider             mock
  Model                echo
```

## Acceptance Criteria

- [ ] `/model` in mock mode opens the model selector or shows the current `mock/echo` model.
- [ ] `/provider` in mock mode opens the provider selector or shows the configured mock provider.
- [ ] `configured_providers()` includes providers that have a valid config entry, even if the API key is empty (as is normal for `mock`).
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `/model` and `/provider` scenarios no longer show the "No connected providers" message.

## Tests

### Layer 1 — State/Logic
- [ ] `mock_provider_is_configured` — `AppState::configured_providers()` includes `mock` with model `echo`.
- [ ] `model_command_with_mock_opens_selector` — `handle_model` with no args and a configured mock provider returns an `OpenDialog(ModelSelector)` result.

### Layer 2 — Event Handling
- [ ] `provider_command_with_mock_opens_selector` — `/provider` event opens the provider dialog.

### Layer 3 — Rendering
- [ ] `model_selector_renders_mock_models` — `TestBackend` shows `echo` in the model selector.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_model_command_shows_selector` — live tmux script runs `/model` and asserts `echo` is selectable.

## Files touched

- `crates/runie-core/src/model/app_state.rs` (or wherever `configured_providers` lives)
- `crates/runie-core/src/commands/dsl/handlers/model.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs` (if `/provider` is there)

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The `mock` provider intentionally has an empty `api_key`. The "connected" check must not require a non-empty key for mock or for providers that do not need authentication.
- This bug makes it impossible to switch models/providers in the TUI.
