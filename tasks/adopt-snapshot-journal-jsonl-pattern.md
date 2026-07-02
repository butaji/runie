# Adopt snapshot + append-only JSONL journal pattern

## Status

`todo`

## Description

Session saves currently rewrite the whole file and header on every append. Switch to an append-only journal with periodic snapshot compaction (jcode/thClaws pattern).

## Acceptance criteria

1. **Unit tests** — Append only adds a line; compaction rebuilds a snapshot without data loss.
2. **E2E tests** — Long replay sessions load correctly after compaction.
3. **Live tmux tests** — Run a long session in tmux, observe quick saves, and resume.

## Tests

### Unit tests
- Append, snapshot, compaction, and recovery.

### E2E tests
- Replay a large session after compaction.

### Live tmux tests
- Use the agent for many turns and save/resume.
