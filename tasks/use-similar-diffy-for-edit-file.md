# Use `similar`/`diffy` for `edit_file`

## Status

`done` — `diffy` is used in `edit_file.rs` for patch creation and validation; unit tests for multi-line edits and ambiguous matches pass.

## Description

`edit_file` uses `replacen(..., 1)` and manual match counting, which is brittle for non-trivial edits. Use `similar` or `diffy` for patch-based edits, or a unified search/replace helper.

## Acceptance criteria

1. **Unit tests** — Patch application produces expected file contents for multi-line edits and rejects ambiguous matches.
2. **E2E tests** — A replay turn with `edit_file` succeeds.
3. **Live tmux tests** — Ask the agent to edit a file in tmux and verify the result.

## Tests

### Unit tests
- Multi-line replacement, no-match, and ambiguous-match cases.

### E2E tests
- Replay fixture applies edits correctly.

### Live tmux tests
- Prompt the agent to refactor a function.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (utility function change; `IoActor` remains authoritative for file IO).
- [ ] **Trigger events:** N/A (edit application doesn't introduce new state transitions).
- [ ] **Observer events:** `FilesWritten` event emitted after edit.
- [ ] **No direct mutations:** Edit must go through `IoActor` for file IO.
- [ ] **No new mirrors:** N/A (utility function change).
- [ ] **Async work observed:** File IO is in `IoActor` via `spawn_blocking`.
