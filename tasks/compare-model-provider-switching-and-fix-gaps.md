# Compare model/provider switching and fix gaps

**Status**: done
**Milestone**: R7
**Category**: Configuration
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-slash-command-model-provider-report-no-providers
**Blocks**: none

## Description

Compare Grok Build's `/model` and provider configuration with Runie's `/model`, `/provider`, and `/settings`. Identify why Runie reports "No connected providers" for the configured mock provider and fix provider detection. Add parity tests.

## Scenario Set

1. Switch model: `/model grok-build-0.1` vs `/model mock/echo`.
2. List available providers/models.
3. Configure a new provider.
4. Launch with missing/invalid provider config and observe error UX.

## Acceptance Criteria

- [x] Each scenario runs in both tools.
- [x] Runie `/model` opens the selector and shows the configured mock model.
- [x] Runie `/provider` shows the configured mock provider.
- [x] `/settings` remains consistent with `/model`/`/provider`.
- [x] Actionable findings become tasks with unit + E2E + live tmux AC.
- [x] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [x] `mock_provider_is_accessible_when_enabled` — `is_mock_enabled()` returns true when mock is set.
  - **Status**: Implemented in `crates/runie-core/src/commands/tests/model.rs`
  - **Test name**: `mock_provider_is_accessible_when_enabled`

### Layer 2 — Event Handling
- [x] `model_mock_enabled_opens_selector_with_echo_model` — `/model` with no args opens the model selector with mock/echo.
  - **Status**: Implemented in `crates/runie-core/src/commands/tests/model.rs`
  - **Test name**: `model_mock_enabled_opens_selector_with_echo_model`
- [x] `provider_dialog_includes_mock_when_enabled` — `/provider` shows mock when enabled.
  - **Status**: Implemented in `crates/runie-core/src/commands/tests/model.rs`
  - **Test name**: `provider_dialog_includes_mock_when_enabled`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `tmux_model_shows_echo` — live tmux script runs `/model` and sees `echo`.
  - **Status**: Covered by existing `scripts/tmux-smoke-test.sh mock` test.

## Files touched

- `crates/runie-core/src/update/dialog/open.rs` — Added mock provider to model selector when enabled.
- `crates/runie-core/src/provider/dialog.rs` — Added mock provider to providers dialog when enabled.
- `crates/runie-core/src/commands/tests/model.rs` — Added Layer 1 and Layer 2 tests.

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for `/model`, `/provider`, and `grok inspect` output. Runie tests compare against the recorded reference; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation. ✅ PASSED
2. **E2E tests** — cover the event handling and/or provider-replay path. ✅ PASSED
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal. ✅ PASSED (existing script)

## Notes

- Overlaps with `fix-slash-command-model-provider-report-no-providers`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `ConfigActor` owns `Config`; provider/model state is authoritative there.
- [x] **Trigger events:** `SwitchModel`, `RunNameCommand` trigger config changes.
- [x] **Observer events:** `ModelSwitched`, `ProviderConfigured` notify observers of config changes.
- [x] **No direct mutations:** Config changes must go through `ConfigActor`; no direct `AppState` mutation.
- [x] **No new mirrors:** Provider/model state in UI must be a projection from events, not a duplicate store.
- [x] **Async work observed:** Provider detection/validation must be awaited or have a JoinHandle owner.

## Implementation Details

### Bug Fixed

When `RUNIE_MOCK` was set but no TOML providers were configured:
1. `/model` opened the selector but showed no models
2. `/provider` showed "No providers configured"

**Root Cause:** `configured_providers()` only returns TOML-configured providers. The mock provider is not in TOML config but is enabled via the `RUNIE_MOCK` env var.

**Fix Applied:**
1. `open_model_selector()` now adds `mock/echo` to the model list when mock is enabled
2. `build_providers_dialog()` now adds the mock provider entry when mock is enabled

This mirrors the existing behavior in `has_any_available_provider()` which already checks for `is_mock_enabled()`.
