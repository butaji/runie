# Add structural message truncation for context window

## Status

`todo`

## Context

Before dropping whole messages, long codeblocks and `<details>` blocks should be shortened structurally (gptme pattern).

## Goal

Add `truncate_msg` that keeps first/last lines of codeblocks/details with `[...]` placeholder; preserve tool pairs atomically.

## Acceptance Criteria
- [ ] Implement structural truncation.
- [ ] Treat tool-call/tool-result pairs as atomic.
- [ ] Add tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for codeblock/details truncation.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Token limit tests pass.
- **Live tmux validation:** Long file reads still parse.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
