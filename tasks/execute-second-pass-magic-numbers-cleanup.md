# Execute second-pass magic numbers cleanup

## Status

**done** — All 6 production code second-pass fixes completed:
- DEFAULT_PERMISSION_TIMEOUT_SECS (60s)
- DEFAULT_MAX_TOOL_ROUNDS (5)
- TOKEN_PREVIEW_LENGTH (8)
- SPAWN_ALLOW_LOOKBACK (10)
- EFFECT_FORWARDER_CHANNEL_CAPACITY (16)
- EVENT_BUS_CHANNEL_CAPACITY (16)

## Description

Track the cleanup of magic numbers introduced in recent commits. Success metrics: zero new unexplained literals in audited files, build.rs comment corrected.

## Acceptance criteria

1. **Unit tests** — Metrics script reports no new bare literals in target files.
2. **E2E tests** — Smoke tests pass after cleanup.
3. **Live tmux tests** — Full manual tmux session shows no regressions.

## Tests

### Unit tests
- Grep check for bare literals in target files.

### E2E tests
- Smoke tests.

### Live tmux tests
- Complete a coding session in tmux.
