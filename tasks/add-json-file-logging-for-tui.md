# Add JSON file logging for TUI

## Status

`todo`

## Description

TUI tracing currently writes to stdout/stderr, which can corrupt the terminal. Add a JSON file appender for TUI mode while preserving console logging for headless/CLI.

## Acceptance criteria

1. **Unit tests** — TUI mode writes structured JSON logs to a file; CLI mode still uses pretty console output.
2. **E2E tests** — Log file is created and contains expected spans.
3. **Live tmux tests** — Run the TUI for a while and inspect the log file.

## Tests

### Unit tests
- Subscriber selection based on mode.

### E2E tests
- TUI launch creates log file.

### Live tmux tests
- Run TUI and check `~/.local/share/runie/logs/`.
