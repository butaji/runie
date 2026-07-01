# Regenerate config schema and fix PermissionMode description

## Status

`todo`

## Context

`config.schema.json:372` lists camelCase values in the `PermissionMode` description while the constants are snake_case. The schema may be stale relative to recent type changes.

## Goal

Regenerate `config.schema.json` from Rust types and verify the description matches constants.

## Acceptance Criteria
- [ ] Run schema generator.
- [ ] Verify `PermissionMode` description uses snake_case.
- [ ] Check no unintended schema diffs.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Schema validation tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
