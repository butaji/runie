# P74 — Memoized TUI Projection Bundle

## Objective

Use the same event-fold boundary for renderer-independent TUI state so the
aggregate snapshot is composed from independent actor-owned projections:

```text
TuiSnapshot = { feed: memo(feed_events), status: memo(status_events),
                prompt: memo(prompt_events), dialog: memo(dialog_events) }
```

## Scope

- Normalize status, prompt, dialog, model-selector, and feed transitions as
  typed events where they are currently command-like mutation messages.
- Keep one reducer and one owner per state domain.
- Publish snapshots through `watch` only after the corresponding event has
  been reduced and acknowledged.
- Remove duplicate compatibility mutation paths only after equivalent snapshot
  and replay coverage exists.
- Add a projection test helper that folds a finite event sequence and exposes
  intermediate states for event-based assertions.

## Acceptance criteria

- YAML fixtures and live EventBus projections produce equivalent snapshots for
  equivalent event sequences.
- Renderers consume snapshots and do not perform state transitions.
- Animation/clock input is represented as explicit events or injected values;
  reducers do not read wall-clock time.
- Tests cover cross-projection ordering and reset behavior.
- Run unit and replay tests, `cargo fmt --all`, and the relevant `cargo`/`just`
  verification commands.

