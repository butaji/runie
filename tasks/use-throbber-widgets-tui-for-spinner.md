# Use throbber-widgets-tui for spinner

## Status

`todo`

## Context

`crates/runie-core/src/model/state/domain_ops.rs`, `labels.rs`, `runie-tui/src/status_bar.rs`, and `theme/glyph.rs` compute a hand-rolled 12-frame braille spinner from `animation_frame % 12`.

## Goal

Replace with `throbber-widgets-tui` (`Throbber`/`ThrobberState`).

## Acceptance Criteria
- [ ] Add dependency.
- [ ] Replace custom frame math.
- [ ] Preserve visual output; update spinner tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Spinner snapshots match.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** Thinking indicator spins.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
