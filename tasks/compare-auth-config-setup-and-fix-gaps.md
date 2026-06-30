# Compare auth/config setup and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Configuration
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Compare Grok Build's `grok login`, `grok inspect`, and `XAI_API_KEY` flow with Runie's `runie-headless inspect`, config.toml, and keyring integration. Identify friction in provider setup and fix with unit + E2E tests.

## Scenario Set

1. Grok `grok login` / browser OAuth.
2. Grok `grok inspect` output.
3. Runie `runie-headless inspect` output.
4. Runie config.toml provider setup.
5. Missing provider config error UX in both tools.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie `inspect` clearly shows config sources, provider, model, and any errors.
- [ ] Missing provider config produces a helpful error message with setup hints.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `inspect_reports_missing_provider` — `runie-headless inspect` with no config shows actionable diagnostics.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_inspect_output_parity` — both tools' inspect commands produce comparable diagnostics.

## Files touched

- `crates/runie-cli/src/inspect.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/config.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for `grok login`, `grok inspect`, and missing-auth error output. Runie tests compare diagnostics against these fixtures; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Focus on first-time setup UX; confusing auth/config is a major adoption barrier.
