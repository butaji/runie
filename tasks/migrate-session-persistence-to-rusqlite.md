# Migrate session persistence to rusqlite

## Status

`todo`

## Context

`session/store.rs` uses JSONL with a header line. `session/persistence/header.rs:40-48` rewrites the header on metadata changes with `File::create` (truncate) then re-reads the body. A crash between truncate and rewrite destroys the entire session. Listing is O(N·M) and durable events drop rich data.

## Goal

Replace JSONL session persistence with `rusqlite` (bundled feature). Store messages, metadata, tree edges, and tool results in a relational schema. One-time import existing `.jsonl` sessions.

## Acceptance Criteria

- [ ] Add `rusqlite` to workspace dependencies.
- [ ] Design schema: `sessions`, `messages`, `message_parts`, `tree_edges` tables.
- [ ] Implement import of existing `.jsonl` sessions.
- [ ] Replace `SessionStore` read/write/list with SQLite operations.
- [ ] Delete `session/persistence/header.rs`, `session/index.rs`, custom locking, and header rewrite logic.
- [ ] All session tests pass.

## Design Impact

No change to TUI element design or composition. Only session persistence behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for schema migrations and import.
- **Layer 2 — Event Handling:** `SessionLoaded`/`SessionListUpdated` facts unchanged.
- **Layer 3 — Rendering:** `/sessions` popup snapshots match.
- **Layer 4 — E2E:** Headless CLI `/load`, `/save`, `/sessions` work after migration.
- **Live tmux testing session (required):** Create, star, rename, resume sessions; verify no data loss across restarts.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
