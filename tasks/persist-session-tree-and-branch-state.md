# Persist session tree and branch state

## Status

`todo`

## Context

`session/mod.rs:32-33` marks the session tree with `#[serde(skip)]`; `session/tree.rs:98-120` has a broken `Clone`. Branch/fork state is lost on save/load/export/import.

## Goal

Persist tree edges and the current branch. With SQLite, add `parent_message_id` to messages. With JSONL, add a durable `SessionTreeSnapshot` event and implement real `Serialize`/`Clone`.

## Acceptance Criteria

- [ ] Tree edges survive save/load.
- [ ] Current branch is restored.
- [ ] `/fork` and branch navigation work after restart.
- [ ] Tests cover forking, navigating, saving, loading.

## Design Impact

No change to TUI element design or composition. Only session tree persistence behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for tree serialization and branch restoration.
- **Layer 2 — Event Handling:** `SessionLoaded` restores tree facts.
- **Layer 3 — Rendering:** Session tree popup shows branches after load.
- **Layer 4 — E2E:** Headless CLI forks a session and reloads it.
- **Live tmux validation:** Fork a session, quit, restart, and verify the branch structure is intact.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
