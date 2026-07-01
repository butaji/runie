# Use textwrap for truncate_args and blockquote wrap

## Status

`todo`

## Context

`crates/runie-core/src/tool/format.rs:55-74` and `runie-tui/src/message/support.rs:195-247` manually iterate characters and accumulate display width to truncate/wrap text.

## Goal

Use `textwrap` (already a dependency) with `WordSeparator`/`WordSplitter`.

## Acceptance Criteria
- [ ] Replace manual truncation/wrapping.
- [ ] Preserve custom width rules.
- [ ] Update snapshots if boundaries shift.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for truncation/wrapping.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests pass.
- **Layer 4 — E2E:** Tool tests pass.
- **Live tmux validation:** Blockquotes render correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
