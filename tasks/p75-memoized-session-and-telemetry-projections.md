# P75 — Memoized Session and Telemetry Projections

## Objective

Apply the pure event-fold model to the two remaining actor-heavy projections:

```text
SessionState   = fold(SessionEvent, reduce_session)
TelemetryState = fold(TelemetryEvent, reduce_telemetry)
```

## Scope

### Session

- Separate journal facts from commands and filesystem effects.
- Make JSONL import, branch reconstruction, compaction, and query behavior
  pure folds over validated session events.
- Keep persistence workers and `SessionActor` mailbox ownership intact.
- Preserve Pi JSONL validation and append-only sequence/parent invariants.

### Telemetry

- Model span start, event, attribute, status, and end as reducer inputs.
- Keep exporter calls as post-reduction effects; an exporter failure must not
  rewind or alter settled actor state.
- Preserve Pi attribute validation, nested span parentage, and replay actions.

## Acceptance criteria

- Session and telemetry replay tests reconstruct identical snapshots from the
  same event sequences.
- Import/export and exporter failure tests remain deterministic and contain no
  sleeps.
- Effects cannot mutate projection state outside the owning actor.
- Full-history replay is available for restore/debugging; live processing uses
  incremental reduction.
- Run unit and replay tests, `cargo fmt --all`, and the relevant `cargo`/`just`
  verification commands.

