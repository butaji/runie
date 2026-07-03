# Scope diff comparison to headless unified output

## Status

`done`

## Context

`compare-diff-edit-output-and-fix-gaps.md` assumes TUI approve/reject parity with Grok Build, but Runie has no such TUI flow. This task documents the scope.

## Goal

The comparison is scoped to headless diff output only: Grok's diff compared against Runie's `Diff::to_unified_string()`. TUI edit approval is a separate feature.

## Changes Made

### `tasks/compare-diff-edit-output-and-fix-gaps.md`
Updated task to clarify scope:

- **Status**: `blocked` (Grok Build fixtures not yet available)
- **Note**: "Runie does not have a TUI approve/reject flow for edits. TUI edit approval is deferred to a separate feature task."
- **Validation**: Live tmux session not required (headless comparison only)

The blocked task already has:
- Scenario set scoped to headless (diff output comparison only)
- Acceptance criteria covering headless diff rendering
- Layer 3 rendering tests and Layer 4 E2E tests
- Fixture/replay strategy (recorded Grok Build fixtures, no live invocation)

## Acceptance Criteria Status

- [x] **Update comparison task acceptance criteria** — already defined in `compare-diff-edit-output-and-fix-gaps.md`
- [x] **Define normalized diff fixture format** — `Diff::to_unified_string()` is the format; fixtures use recorded Grok Build output
- [x] **Defer TUI edit approval to a separate feature task** — documented in task note

## SSOT/Event Compliance

- [x] **Actor/SSOT:** N/A (documentation/scoping change).
- [x] **Trigger events:** N/A.
- [x] **Observer events:** N/A.
- [x] **No direct mutations:** N/A.
- [x] **No new mirrors:** N/A.
- [x] **Async work observed:** N/A.
