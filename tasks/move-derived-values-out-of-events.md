# Move derived values out of events

## Status

`todo`

## Description

Speed (`speed_tps`), duration (`duration_secs`), and compaction ratio should be computed in projection code from raw facts, not carried in events.

## Acceptance criteria

1. **Unit tests** — `TokenStatsUpdated`, `TurnComplete`, `CompactionTriggered` carry only raw facts; derived values match old behavior.
2. **E2E tests** — Replay still computes the same derived values.
3. **Live tmux tests** — Run a turn and verify speed/duration display.

## Tests

### Unit tests
- Derived values computed from raw facts.

### E2E tests
- Replay computes same values.

### Live tmux tests
- Observe tokens-per-second and turn duration.
