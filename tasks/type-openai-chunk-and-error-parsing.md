# Type OpenAI chunk and error parsing

## Status

`todo`

## Context

`parse_chunk`, `parse_tool_call_deltas`, and `parse_error_value` manually traverse `serde_json::Value` and classify errors by substring.

## Goal

Use strongly typed structs for OpenAI streaming chunks and errors, or adopt `async-openai` types.

## Acceptance Criteria
- [ ] Define typed chunk/error structs with `serde::Deserialize`.
- [ ] Replace manual `Value` navigation.
- [ ] Preserve MiniMax-specific fields.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for chunk/error deserialization.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** OpenAI and MiniMax fixture replay tests pass.
- **Live tmux validation:** Real provider streaming works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
