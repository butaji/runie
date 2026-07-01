# Use tui-scrollview for message feed

## Status

`todo`

## Context

`crates/runie-tui/src/ui/scroll.rs`, `ui/messages/`, `message/wrap.rs`, and `message/support.rs` implement scrollable message feeds and blockquote wrapping by hand, maintaining parallel line-count math in `layout.rs`.

## Goal

Adopt `tui-scrollview` (`ScrollView`) to reduce custom wrapping/scroll logic.

## Acceptance Criteria
- [ ] Add dependency.
- [ ] Render message feed inside `ScrollView`.
- [ ] Coordinate with `layout::element_line_count` or replace it.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Message list snapshots unchanged.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** Scroll long conversations.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
