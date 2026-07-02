# Replace grep/find shell-outs with `walkdir`/`ignore`/`regex`

## Status

`todo`

## Description

`grep` and `find` tools shell out to `rg`/`grep`/`fd`/`find`, which is cross-platform fragile and duplicates command-building logic. Use `walkdir`/`ignore` + `regex` (or the `grep` crate) instead.

## Acceptance criteria

1. **Unit tests** — Results match old shell-out behavior for representative queries.
2. **E2E tests** — Mock-provider turns using grep/find produce the same output.
3. **Live tmux tests** — Run `grep` and `find` tools in tmux on a real repo.

## Tests

### Unit tests
- Regex matching and directory traversal match old outputs.

### E2E tests
- Replay turn exercises grep and find.

### Live tmux tests
- Ask the agent to grep a pattern and find files.
