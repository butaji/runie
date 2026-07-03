# Compare diff and edit output and fix gaps

**Status**: wontfix
**Blocked reason**: Grok Build fixtures not present; comparison harness not yet built.

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo)
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Compare how Grok Build and Runie produce unified diff output for file edits. Grok Build shows clean unified diffs. Runie has diff rendering via `Diff::to_unified_string()`. Identify gaps and fix with unit + E2E tests.

> **Note**: Runie does not have a TUI approve/reject flow for edits. TUI edit approval is deferred to a separate feature task.

## Scenario Set (Headless)

1. Record Grok Build diff output for an edit prompt.
2. Compare against Runie's `Diff::to_unified_string()` for the same file change.
3. Identify rendering gaps.

## Acceptance Criteria

- [ ] Headless diff comparison scenario defined.
- [ ] Runie `Diff::to_unified_string()` produces readable unified diffs.
- [ ] Gaps documented and actionable findings become separate tasks.
- [ ] `cargo test --workspace` passes after any fixes.

## Tests

### Layer 3 — Rendering
- [ ] `diff_renders_unified_change` — `TestBackend` shows `+` / `-` lines correctly.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `diff_unified_string_matches_expected` — `Diff::to_unified_string()` produces expected format.

## Files touched

- `crates/runie-tui/src/diff.rs`
- `crates/runie-core/src/diff/mod.rs`
- `crates/runie-core/src/tools/edit.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for diff output. Runie tests validate against the recorded diff format; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — diff rendering smoke test in tmux (deferred to separate task if no TUI flow exists).

## Notes

- Runie does not have a TUI approve/reject flow for edits; this is a separate feature.
- Focus is on headless/unified output comparison, not TUI interaction.
> **Live tmux testing session required:** N/A (headless comparison only; TUI flow deferred).
