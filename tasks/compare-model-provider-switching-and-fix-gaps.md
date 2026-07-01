# Compare model/provider switching and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Configuration
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-slash-command-model-provider-report-no-providers
**Blocks**: none

## Description

Compare Grok Build's `/model` and provider configuration with Runie's `/model`, `/provider`, and `/settings`. Identify why Runie reports “No connected providers” for the configured mock provider and fix provider detection. Add parity tests.

## Scenario Set

1. Switch model: `/model grok-build-0.1` vs `/model mock/echo`.
2. List available providers/models.
3. Configure a new provider.
4. Launch with missing/invalid provider config and observe error UX.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie `/model` opens the selector and shows the configured mock model.
- [ ] Runie `/provider` shows the configured mock provider.
- [ ] `/settings` remains consistent with `/model`/`/provider`.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `mock_provider_is_configured` — `configured_providers()` includes mock with empty api_key.

### Layer 2 — Event Handling
- [ ] `model_command_opens_selector` — `/model` with no args opens the model selector.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_model_shows_echo` — live tmux script runs `/model` and sees `echo`.

## Files touched

- `crates/runie-core/src/model/app_state.rs`
- `crates/runie-core/src/commands/dsl/handlers/model.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for `/model`, `/provider`, and `grok inspect` output. Runie tests compare against the recorded reference; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Overlaps with `fix-slash-command-model-provider-report-no-providers`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
