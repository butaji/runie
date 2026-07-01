# Add session compaction with threshold and tool-pair summarization

## Status

`todo`

## Context

Runie has no automatic context-window management. Goose uses a configurable threshold, progressive tool-response removal, and async tool-pair summarization.

## Goal

Implement compaction triggered by token ratio, hide old tool messages via `MessageOrigin`, and summarize tool-call/response pairs off the hot path.

## Acceptance Criteria
- [ ] Add `Compaction` origin and compaction event.
- [ ] Trigger compaction at configurable context-limit ratio.
- [ ] Summarize tool pairs asynchronously.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for compaction strategy.
- **Layer 2 — Event Handling:** Compaction facts emitted.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Long conversation replay tests pass.
- **Live tmux validation:** Very long chat does not crash.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
