# Compare diff and edit output and fix gaps

**Status**: blocked

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo), Grok Build fixtures not present
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Compare how Grok Build and Runie present file edits and diffs. Grok Build shows clean unified diffs and an approve/reject flow. Runie has diff rendering but it may be unclear or incomplete. Identify gaps and fix with unit + E2E tests.

## Scenario Set

1. Prompt for a small edit: `"add a doc comment to src/lib.rs"`.
2. Observe diff rendering in Grok Build.
3. Observe diff rendering in Runie.
4. Approve or reject the edit.

## Acceptance Criteria

- [ ] Each scenario runs in both tools in a temp repo.
- [ ] Runie diff output is readable and matches the intended change.
- [ ] Approve/reject flow (if present) is navigable.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 3 — Rendering
- [ ] `diff_renders_unified_change` — `TestBackend` shows `+` / `-` lines correctly.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_edit_diff_parity` — both tools produce comparable diff output for the same prompt.

## Files touched

- `crates/runie-tui/src/diff.rs`
- `crates/runie-core/src/diff/mod.rs`
- `crates/runie-core/src/tools/edit.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for edit prompts, diff output, and approve/reject flows. Runie tests validate against the recorded diff format; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This may surface that Runie lacks an edit tool or approval flow for edits.
