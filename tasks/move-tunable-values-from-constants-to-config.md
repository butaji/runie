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

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ConfigActor` owns config values; constants become configurable.
- [ ] **Trigger events:** Config load events trigger value updates.
- [ ] **Observer events:** Config changes propagate via existing events.
- [ ] **No direct mutations:** Config values must be set through `ConfigActor`, not direct mutation.
- [ ] **No new mirrors:** Config values are authoritative in `ConfigActor`; no duplicate storage.
- [ ] **Async work observed:** Config loading is synchronous; no new async work introduced.
