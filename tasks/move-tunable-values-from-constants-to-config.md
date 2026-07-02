# Move tunable values from constants to config

## Status

`todo`

## Description

Some literals are constants but should be user-configurable: HTTP timeouts, retry policy, event-bus capacity, FFF scan timeout/result limit, transient message timeout, tool result limits. Move them to the config schema where appropriate.

## Acceptance criteria

1. **Unit tests** — Config defaults match previous constant values; overrides are applied.
2. **E2E tests** — Smoke tests pass with default and overridden config.
3. **Live tmux tests** — Override a timeout/limit via env/config and verify behavior in tmux.

## Tests

### Unit tests
- Config deserialization and default values.

### E2E tests
- Bootstrap with modified config.

### Live tmux tests
- Set `RUNIE_` env overrides and launch.
