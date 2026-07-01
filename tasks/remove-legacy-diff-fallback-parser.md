# Remove legacy diff fallback parser

## Status

`todo`

## Context

`crates/runie-core/src/diff/mod.rs:213-269` keeps `fallback_parse_diff` for imperfect agent output even though the parser-removal task is marked done.

## Goal

Delete the fallback or replace it with `similar` if lenient parsing is still required.

## Acceptance Criteria
- [ ] Audit fixtures/tests depending on fallback.
- [ ] Delete fallback or switch to `similar`.
- [ ] Update tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for diff application.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Diff widget snapshots unchanged.
- **Layer 4 — E2E:** Provider replay with diff tool passes.
- **Live tmux validation:** File edit diff applies correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
