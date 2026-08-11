# Reduction 08: immutable snapshot sharing

Status: adopted

`runie_core::EventMemo` provides shared event-log storage and incremental
projection state. Compatibility-owned snapshot channels remain available for
serialization and mutation boundaries; renderer and hot-path consumers use
immutable shared projections.

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
clones with a test-local counting allocator. The source audit covers feed,
status, prompt, UI, model/provider registries, session, agent state, queues,
jobs, todos, plugins, and telemetry; compatibility clones are retained only
at explicit ownership/serialization boundaries. This task is adopted.
Feed snapshot assembly now has one semantic projection module, including its
macro-backed navigation field schema and derived selection projection.
Model catalog and provider registry actors now publish immutable shared
projections alongside their compatibility-owned watch channels, so registry
consumers can avoid deep snapshot clones without an API break.
The top-level `UiActor` now follows the same shared projection contract,
allowing view consumers to read immutable UI state without cloning it while
retaining the existing compatibility subscription.
`SessionActor` now uses one publisher value to fan out every journal state
transition to both its owned compatibility channel and immutable shared
channel; all existing worker publication sites retain the same reducer shape.
`AgentStateActor` now publishes the same immutable shared projection alongside
its compatibility channel, covering the core agent-state boundary without
duplicating event reduction logic.
`TodoActor` now publishes an immutable shared todo projection alongside its
owned compatibility snapshot, keeping validation and replacement reduction
unchanged.
`BackgroundProcessActor` now uses one publisher value for owned and shared job
snapshots, preserving the existing lifecycle reducer while removing duplicate
publication wiring from start, cancel, and completion paths.
`PluginHost` now exposes the `ReducerActor`'s immutable runtime projection,
keeping plugin lifecycle state on the existing event reducer boundary.
`LoopActor` now publishes an immutable shared control projection alongside its
compatibility watch state, keeping abort/run/queue mode transitions on one
reducer-owned data boundary.
The TUI model-selector read paths now consume the model catalog's shared
projection, avoiding repeated deep catalog clones while retaining owned
snapshot APIs for mutation and serialization boundaries.
Provider summary and provider-model selection reads now use the provider
registry's shared projection as well; configuration persistence continues to
use the owned snapshot boundary.
Scrollback fold, range-selection, and copy-selection reads now use the feed
shared projection after their actor acknowledgements, avoiding compatibility
snapshot clones on those command paths.
Palette and model-selector query/activation reads now use the UI actor's shared
projection, while UI event delivery remains mailbox-owned.
The `/copy` command now derives assistant text from the shared feed projection;
session export/clone/compaction continue to use owned snapshots intentionally.
`FollowUpQueueActor` now publishes its previously unused queue snapshot as both
an owned compatibility view and immutable shared projection, with push/drain/
clear transitions reducing through one queue-owned publication boundary.
`SteeringQueueActor` now follows the same queue snapshot contract, keeping both
interactive queue state machines symmetric and renderer-independent.
`TelemetryActor` now exposes the same immutable shared subscription handle as
the other watch-backed actors, with coverage proving subscription and direct
snapshot reads observe one projection.
