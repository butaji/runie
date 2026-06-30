# Use include_dir for MiniMax fixtures

## Status

`todo`

## Context

`crates/runie-testing/src/fixtures/minimax.rs:6-23` defines fixtures with a macro, then a manual `match` maps names to constants. This is boilerplate.

## Goal

Use `include_dir!` over `src/fixtures/minimax/` plus a `LazyLock<HashMap<&str, &str>>`, or expose constants directly and delete `fixture()`.

## Acceptance Criteria

- [ ] Add `include_dir` if not already available, or use `include_str!` + `LazyLock`.
- [ ] Delete the macro and manual match.
- [ ] All fixture consumers compile.

## Design Impact

No change to TUI element design or composition. Only test fixture loading changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all expected fixture names are loadable.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay tests using MiniMax fixtures pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
