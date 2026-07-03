# Add turn journal phases for crash recovery

## Status

`todo`

## Description

Introduce durable turn-journal phases (`TurnStarted`, `ProviderCalled`, `ToolRequestsRecorded`, `ResponseDelta`, `TurnCommitted`/`TurnAborted`) so interrupted turns can be reconciled.

## Acceptance criteria

1. **Unit tests** — Each phase round-trips through durable storage.
2. **E2E tests** — Simulated crash/replay recovers an interrupted turn.
3. **Live tmux tests** — Kill the app mid-turn and resume.

## Tests

### Unit tests
- Phase serialization and replay.

### E2E tests
- Crash recovery replay.

### Live tmux tests
- Kill and resume mid-turn.
