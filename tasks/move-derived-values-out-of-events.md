# Move derived values out of events

## Status

`done`

## Description

`speed_tps` (tokens/sec) is now computed in the projection layer from `speed_window.speed()`, not carried in the event.

## Changes

1. **Event definition** — Removed `speed_tps: f64` from `TokenStatsUpdated` in `event/mod.rs`.

2. **TurnActor** — `handle_update_speed` no longer computes `speed_tps` for the event. The event only carries `tokens_in` and `tokens_out`.

3. **Projection** — `apply_token_stats` now computes `speed_tps` via `speed_window.speed()` from the ring buffer (populated by `record()` calls in TurnActor).

4. **Dispatch** — Updated to not pass `speed_tps` to `apply_token_stats`.

5. **Tests** — Updated all test files to not use `speed_tps` in `TokenStatsUpdated`.

## Acceptance criteria

1. ✅ **Unit tests** — All projection tests pass.
2. ✅ **E2E tests** — Replay still computes speed via ring buffer.
3. ✅ **Live tmux tests** — Speed display works (verified manually).

## Tests

- `apply_token_stats_updates_stats` — verifies tokens updated, speed from empty window = 0.0
