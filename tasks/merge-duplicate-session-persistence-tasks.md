# Merge duplicate session persistence tasks

## Status

`done`

## Description

`adopt-snapshot-journal-jsonl-pattern.md`, `replay-sessions-via-events-through-appstate.md`, and `use-atomic-writes-for-config-and-session-files.md` all fall under the JSONL persistence umbrella. All referenced tasks are already complete, so this merge task is a no-op closure.

## Status of referenced tasks

| Task | Status |
|------|--------|
| `adopt-snapshot-journal-jsonl-pattern.md` | done |
| `replay-sessions-via-events-through-appstate.md` | done |
| `use-atomic-writes-for-config-and-session-files.md` | done |
| `standardize-session-persistence-on-jsonl.md` | done |

All session persistence work has been completed. JSONL pattern is the standard.

## Acceptance criteria

1. **Unit tests** — N/A; backlog task.
2. **E2E tests** — N/A; backlog task.
3. **Live tmux tests** — N/A; backlog task.

## Completion Validation

- [x] **All referenced tasks are done** — No merge needed; work is complete.
