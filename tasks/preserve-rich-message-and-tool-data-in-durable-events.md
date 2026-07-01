# Preserve rich message and tool data in durable events

## Status

`todo`

## Context

`event/durable.rs:14-33` and `event/to_durable.rs:46-62` flatten messages to `content: String`, dropping `Part`s (images, tool calls, reasoning). `ToolResult` hardcodes `success: true` and drops `duration`.

## Goal

Store the full `ChatMessage.parts` vector in durable events (or SQLite `messages.parts_json`). Preserve tool success and duration.

## Acceptance Criteria

- [ ] Durable events include `parts` (or equivalent JSON) instead of flat `content`.
- [ ] Tool results include success/failure and duration.
- [ ] Existing sessions can be imported/migrated.
- [ ] Replay produces the same conversation.

## Design Impact

No change to TUI element design or composition. Only durable event format changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for durable event round-trip of rich messages and tool results.
- **Layer 2 — Event Handling:** Replay events carry full data.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session replay with images/reasoning/tools passes.
- **Live tmux testing session (required):** A turn with tool calls and images survives save/load.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
