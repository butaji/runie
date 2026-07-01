# Scope diff comparison to headless unified output

## Status

`todo`

## Context

`compare-diff-edit-output-and-fix-gaps.md` assumes TUI approve/reject parity with Grok Build, but Runie has no such TUI flow. The task is blocked.

## Goal

Scope the comparison to headless diff output: record Grok's diff and compare against Runie's `Diff::to_unified_string()`.

## Acceptance Criteria
- [ ] Update comparison task acceptance criteria.
- [ ] Define normalized diff fixture format.
- [ ] Defer TUI edit approval to a separate feature task.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Diff comparison scenario passes headlessly.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
