# Centralize tool default limits

## Status

`todo`

## Description

`grep`/`find`/`find_definitions` tools use hardcoded default limits (`100`, `10`, `5`, `200`) and depth values. Centralize these in `runie-agent::tool::constants`.

## Acceptance criteria

1. **Unit tests** — Tool defaults are named constants and covered by unit tests.
2. **E2E tests** — Mock-provider tool calls still produce the same result counts.
3. **Live tmux tests** — Ask the agent to grep/find/define in tmux and verify result limits.

## Tests

### Unit tests
- Default limit constants exist and are used.

### E2E tests
- Replay fixtures exercise grep/find/definitions.

### Live tmux tests
- Run `grep` and `find` tools on a real repo.
