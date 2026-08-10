# Reduction 10: event-trace harness

Status: partial

Create one event-sequence/YAML harness usable by core actors, TUI projections,
and visual state tests.

Progress: `event_trace!` now expresses ordered pure reducer traces with minimal
boilerplate while retaining each event at the call site. YAML fixture
integration now uses the public `replay_yaml` helper in `runie-core`, which
parses an ordered event sequence and reuses `EventMemo` replay semantics.
The core integration suite now replays a checked-in YAML fixture through the
public API. Broader TUI fixture adoption remains future work.
Feed tool-record tests also use `event_trace!` directly, covering the
memoized projection path across multiple domain events.
Status projection tests now cover both explicit `EventMemo::replay` and the
`event_trace!` shorthand, asserting identical ordered events and snapshots.
`Status` and `StatusMsg` now have serde representations, and a YAML status
trace exercises the shared public `replay_yaml` helper end to end.
Paint document round-trip coverage now consumes a checked-in YAML fixture,
extending fixture-backed declarative data into renderer-neutral paint values.

Acceptance: representative core, feed, status, and TUI tests use the harness;
no sleeps are introduced.
