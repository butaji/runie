# Compare interactive TUI launch scenarios and fix gaps

**Status**: blocked

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo), Grok Build fixtures not present
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Run Grok Build TUI and Runie TUI in tmux for launch, welcome, simple chat, and slash-help scenarios. Compare welcome clarity, hint text, status bar, and error handling. Fix Runie gaps with unit + E2E tests.

## Scenario Set

1. Launch and welcome screen.
2. Type `hello` and submit.
3. Type `/help` and submit.
4. Type `/quit` and submit.
5. Launch with invalid/missing provider config.

## Acceptance Criteria

- [ ] Each scenario is captured from both TUIs.
- [ ] Differences are classified and documented.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] The mock `hello` repetition and stuck `Working...` issues are resolved before comparing chat output.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 3 — Rendering
- [ ] `tui_welcome_renders_clear_hints` — `TestBackend` snapshot matches expected welcome text.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_tui_launch_parity` — both TUIs show a usable prompt within 5s.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/ui.rs`
- `crates/runie-tui/src/popups/welcome.rs`

## Fixture / Replay Strategy

This task must use recorded Grok Build TUI pane fixtures (`crates/runie-testing/fixtures/grok-build/tui/`) produced by `scripts/record-grok-build-fixtures.sh`. Derive `TestBackend` expected buffers from the pane dumps. Do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Focus on first-run UX; a dead-end here prevents users from ever using Runie.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
