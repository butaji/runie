# Regenerate index and mark typed ChatMessage builder done

## Status

`done`

## Context

`tasks/typed-chatmessage-builder-and-shrink-sanitize.md` is implemented (`ChatMessageBuilder` exists in `proto/message/mod.rs`, `sanitize.rs` shrunk from 333 to 134 LOC), but its file status is still `todo` and `tasks/index.json` lists it as `todo`.

## Goal

Mark the task file `done` and regenerate `tasks/index.json`.

## Acceptance Criteria
- [ ] Change the task file status to `done`.
- [ ] Regenerate `tasks/index.json`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
