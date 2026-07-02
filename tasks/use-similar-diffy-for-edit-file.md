# Use `similar`/`diffy` for `edit_file`

## Status

`todo`

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
