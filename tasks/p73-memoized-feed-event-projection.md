# P73 — Memoized Feed Event Projection

## Objective

Make the TUI feed a pure event projection:

```text
FeedState = fold(FeedEvent, reduce_feed)
```

The `ScrollbackActor` remains the sole owner. It accepts validated commands,
appends or receives ordered feed events, incrementally reduces its state, and
publishes immutable `FeedSnapshot` values.

## Scope

- Define a typed `FeedEvent` vocabulary for the existing `ScrollbackMsg`
  transitions.
- Extract `reduce_feed(state, event) -> state` from the current reducer while
  preserving event ordering and actor ownership.
- Keep rendering, wrapping, styling, and Ratatui buffer construction outside
  the reducer.
- Make YAML replay and live EventBus projection use the same event-to-state
  path.
- Retain a bounded event history or replay hook for deterministic diagnostics;
  do not recompute the full history for every event.

## Acceptance criteria

- Existing feed behavior and snapshots are unchanged.
- Replay tests can reconstruct a `FeedSnapshot` from an event sequence without
  starting a terminal renderer.
- Tests cover tool start/update/end, parallel tool identity, fold/navigation,
  assistant streaming, clear/reset, and event ordering.
- No actor mutates another actor's state directly.
- Run unit and replay tests, `cargo fmt --all`, and the relevant `cargo`/`just`
  verification commands.

