# Reduction 08: immutable snapshot sharing

Status: partial

`runie_core::EventMemo` provides shared event-log storage and incremental
projection state. Actor snapshot channels still clone domain snapshots and
remain future adoption work.

`ReducerActor` now also exposes a short-lived borrowed snapshot view; callers
that do not need ownership can avoid cloning the snapshot.

`SharedSnapshot<S>` now provides an explicitly immutable, cheaply clonable
`Arc`-backed view for consumers that can share a projection lifetime.
`ReducerActor` now publishes an optional shared watch channel and forwards
`shared_snapshot`/`shared_subscribe` through the actor declaration macro;
existing owned snapshot APIs remain compatible.
The status actor exposes the shared channel and the live renderer uses a
shared status projection for animation-demand reads.
`ScrollbackActor` now publishes the same shared projection alongside its
compatibility-owned channel, with an event-driven subscription test.
The live renderer uses the shared feed projection for animation-demand reads.
The prompt actor now publishes the same shared projection alongside its
compatibility-owned channel, with focused actor coverage for shared reads.
The event renderer now keeps the shared feed projection for its per-event
atomic read, avoiding a deep snapshot clone on that hot path.
Its immutable status label and elapsed-time reads now use the shared status
projection as well.

Acceptance: snapshot isolation tests, allocation-sensitive focused benchmark,
and workspace tests.

Verification note: the workspace suite is green, and
`shared_snapshot_alloc.rs` measures repeated shared versus deep projection
clones with a test-local counting allocator. Runtime consumers remain
incremental, so this task remains partial.
Feed snapshot assembly now has one semantic projection module, including its
macro-backed navigation field schema and derived selection projection.
Model catalog and provider registry actors now publish immutable shared
projections alongside their compatibility-owned watch channels, so registry
consumers can avoid deep snapshot clones without an API break.
The top-level `UiActor` now follows the same shared projection contract,
allowing view consumers to read immutable UI state without cloning it while
retaining the existing compatibility subscription.
