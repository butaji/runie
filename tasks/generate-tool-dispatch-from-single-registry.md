# Generate tool dispatch from a single registry

## Status

`todo`

## Description

`dispatch_tool` and `BUILTIN_TOOL_NAMES` are hand-maintained lists. Generate both from a single registry using a macro or `inventory`/`linkme`.

## Acceptance criteria

1. **Unit tests** — Adding a tool requires changing only one source of truth; name list and dispatch stay in sync.
2. **E2E tests** — All built-in tools still execute correctly in mock-provider replay.
3. **Live tmux tests** — Run bash, read, grep, find, and edit tools in tmux.

## Tests

### Unit tests
- Registry contains all built-in tools; dispatch routes to the right implementation.

### E2E tests
- Multi-tool replay turn uses every built-in tool.

### Live tmux tests
- Submit a prompt that triggers read/grep/edit/bash tools.
