# Create Grok Build fixture recorder and record fixtures

**Status**: wontfix — Grok Build binary unavailable. Proceeds with documented behavior.
**Milestone**: R7
**Category**: Testing
**Priority**: P0

**Depends on**: prepare-grok-build-reference-for-comparison, build-runie-vs-grok-build-comparison-harness
**Blocks**: compare-headless-one-shot-scenarios-and-fix-gaps, compare-interactive-tui-launch-scenarios-and-fix-gaps, compare-tool-execution-permission-flows-and-fix-gaps, compare-session-persistence-resumption-and-fix-gaps, compare-plan-mode-and-context-workflows-and-fix-gaps, compare-file-context-picker-and-fix-gaps, compare-model-provider-switching-and-fix-gaps, compare-diff-edit-output-and-fix-gaps, compare-subagent-mcp-support-and-fix-gaps, compare-auth-config-setup-and-fix-gaps, compare-quit-abort-error-recovery-and-fix-gaps, compare-multi-turn-conversation-and-fix-gaps

## Description

Build a one-time recorder script that runs Grok Build for every comparison scenario, captures the outputs, and stores them as fixtures. Subsequent Runie tests and the comparison harness must use these fixtures, not live Grok invocations.

## Acceptance Criteria

- [ ] `scripts/record-grok-build-fixtures.sh` exists and is executable.
- [ ] The script uses a scenario manifest (e.g. `scripts/grok-build-scenarios.toml` or JSON).
- [ ] For each scenario it records stdout, stderr, exit code, and (for TUI) `tmux capture-pane` output.
- [ ] Fixtures are stored under `crates/runie-testing/fixtures/grok-build/`.
- [ ] The script supports `--scenarios <filter>` to re-record a subset.
- [ ] If Grok Build is not authenticated, the script exits with a clear message and does not produce invalid fixtures.
- [ ] The script never writes into `/Users/admin/Code/GitHub/runie-dev`; it uses temp copies.
- [ ] A README in the fixture directory explains how and when to re-record.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `fixture_recorder_produces_expected_files` — dry-run the recorder with a mock command and assert the fixture directory layout is correct.
- [ ] `recorded_hello_fixture_exists` — after recording, the `hello` scenario fixture is non-empty and parseable.

## Files touched

- `scripts/record-grok-build-fixtures.sh` (new)
- `scripts/grok-build-scenarios.toml` (new)
- `crates/runie-testing/fixtures/grok-build/` (new)
- `crates/runie-testing/fixtures/grok-build/README.md` (new)

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is the only task that should invoke live Grok Build for the comparison work.
- All other comparison tasks depend on these fixtures and replay them.
- Re-recording should be a rare, deliberate operation.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (recorder doesn't introduce actor state).
- [ ] **Trigger events:** N/A (recorder doesn't emit events).
- [ ] **Observer events:** N/A (recorder doesn't observe events).
- [ ] **No direct mutations:** Recorder uses temp copies; doesn't mutate source repo.
- [ ] **No new mirrors:** Fixtures are test data, not authoritative state.
- [ ] **Async work observed:** N/A (recorder is a script, not async Rust).
