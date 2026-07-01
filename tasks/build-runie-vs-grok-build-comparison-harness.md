# Build Runie vs Grok Build comparison harness

**Status**: todo
**Milestone**: R7
**Category**: Testing
**Priority**: P0

**Depends on**: prepare-grok-build-reference-for-comparison
**Blocks**: compare-headless-one-shot-scenarios-and-fix-gaps, compare-interactive-tui-launch-scenarios-and-fix-gaps

## Description

Create a reusable script `scripts/compare-with-grok-build.sh` that runs the same scenario in both Grok Build and Runie, captures outputs, and produces a side-by-side diff report. The harness must isolate file mutations to temp copies.

## Acceptance Criteria

- [ ] `scripts/compare-with-grok-build.sh <scenario> [fixture-repo]` exists and is executable.
- [ ] The script clones the fixture repo to a temp directory for each run.
- [ ] It runs the Grok Build command (headless or TUI via tmux) and captures stdout/stderr or pane contents.
- [ ] It runs the equivalent Runie command (headless or TUI via tmux) and captures stdout/stderr or pane contents.
- [ ] It produces a markdown report under `docs/superpowers/plans/2026-06-30-runie-vs-grok-build-findings.md` (or per-scenario files).
- [ ] Scenarios are parameterized so adding a new comparison requires only a small data entry.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_hello_scenario_runs_both_tools` — the harness runs `hello` in both tools and produces output files.
- [ ] `harness_does_not_mutate_fixture_repo` — verify the original fixture repo is unchanged after the harness runs.

## Files touched

- `scripts/compare-with-grok-build.sh` (new)
- `docs/superpowers/plans/2026-06-30-runie-vs-grok-build-findings.md` (new)

## Fixture / Replay Strategy

The harness must compare Runie against recorded Grok Build fixtures (`crates/runie-testing/fixtures/grok-build/`) produced by `scripts/record-grok-build-fixtures.sh`. It must not invoke live Grok Build during normal test runs or CI. Add a `--live` flag for on-demand re-recording only.

See `docs/superpowers/plans/2026-06-30-grok-build-fixture-strategy.md`.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The harness should prefer headless mode for deterministic assertions and use tmux only for TUI-specific scenarios.
- Keep the script under 500 lines; extract helpers if needed.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
