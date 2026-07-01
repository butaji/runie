# Use ansi_colours for theme ANSI256 mapping

## Status

`todo`

## Context

`crates/runie-tui/src/theme/loader.rs` hard-codes ANSI16 palette and 256-color cube/gray-ramp formulas.

## Goal

Use `ansi_colours::rgb_from_ansi256` for all 0-255 indices and delete custom tables.

## Acceptance Criteria
- [ ] Delete custom `ansi16_to_opaline`, cube, gray functions.
- [ ] Use `ansi_colours::rgb_from_ansi256`.
- [ ] Snapshots unchanged.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests compare old/new RGB mappings.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Theme snapshot tests pass.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** Theme renders correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
