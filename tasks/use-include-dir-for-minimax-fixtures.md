# Use include_dir for MiniMax fixtures

## Status

`done`

**Completed:** 2026-06-30

## Context

`crates/runie-testing/src/fixtures/minimax.rs:6-23` defines fixtures with a macro, then a manual `match` maps names to constants. This is boilerplate.

## Goal

Use `include_dir!` over `src/fixtures/minimax/` plus a `LazyLock<HashMap<&str, &str>>`, or expose constants directly and delete `fixture()`.

## Acceptance Criteria

- [x] Add `include_dir` dependency.
- [x] Use `include_dir!` for directory scanning and `LazyLock` for lazy loading.
- [x] All fixture consumers compile and tests pass.

## Design Impact

No change to TUI element design or composition. Only test fixture loading changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all expected fixture names are loadable.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay tests using MiniMax fixtures pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (fixture loading is test-only).
