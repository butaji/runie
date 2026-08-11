# Reduction 04: normalized feed state

Status: adopted

Store canonical feed records and derive tool/activity/navigation projections;
remove synchronized duplicate indexes where possible.

Progress: the feed actor now retains the reduced event sequence through
`EventMemo`; transcript lines remain the canonical ordered record stream while
`FeedFacts` owns normalized tool/activity/workflow metadata and derived card
projections are rebuilt from those two explicit data boundaries.

The first migration boundary is now present: `FeedFacts` is a typed normalized
projection for tool, activity, workflow, and lifecycle facts, populated once
when producing `FeedSnapshot`. The flattened fact fields have been removed
from `FeedSnapshot`; `FeedFacts` is the sole facts boundary. `FeedNavigation`
now owns the nested facts value, and all reducer writers plus scrollback
readers use it.

Tool names and arguments are now unified in one `ToolRecord` registry, while
name and argument events remain independently updatable.
Tool-card projections consume that registry through `ToolNameLookup`, so they
no longer rebuild a temporary parallel name map per snapshot.
The lookup contract is isolated in `feed_tool_lookup.rs`, keeping the tool
projection module below the structural file-size limit.
An event-sequence regression test covers name set, argument set/remove, and
clear behavior for one normalized record.
Lifecycle fields that had remained duplicated on `FeedNavigation` were
removed; `FeedFacts` is now their sole owner and snapshot rehydration uses that
single source.
Activity and workflow reset operations are now methods on `FeedFacts`, so
clear/reset reducers no longer duplicate field-by-field projection logic. The
two reset field groups are declared through one small typed macro, keeping the
normalized ownership table inspectable as data.
The full facts clear operation is also centralized on `FeedFacts`, leaving the
feed reducer's clear path responsible only for transcript and navigation data.

`ToolRecord` now owns name/argument mutation operations, so reducer stages no
longer duplicate record construction and argument clearing semantics.

The live scrollback event projection now stores one normalized `ToolRecord`
per call ID for name and arguments instead of parallel name/argument maps.
`FeedSnapshot` no longer copies the navigation-owned tool-mode map; tool display
mode now has one canonical owner and tool-card projection reads it directly.
No mutable parallel tool-name/argument or lifecycle index remains; line
identity is retained as transcript data and tool metadata is retained in the
typed `ToolRecord` registry.
Tool-header and tool-line classification is now owned by `LineKind` predicates,
so feed update, selection, error, and rendering paths share one vocabulary.
Live-header classification (`Tool`/`ToolRunning`) is also centralized, keeping
settled errors distinct while removing repeated matches from tool-card and
selection projections.
Transcript-selectability is also a `LineKind` predicate, so navigation does
not maintain a separate classification table.
Append/reset, layout measurement, and scroll transitions now share the
semantic `feed_view_state.rs` boundary.
An interleaved lifecycle regression now verifies tool-card ordering and stale
row absence across update, settle, and concurrent insert events.
Logical tool-member ordinals are now one pure transcript projection shared by
navigation lookup and tool-card rows, removing duplicate temporary index maps
and keeping selection/card identity on the same derived data.
The TUI snapshot handoff now transfers the canonical `FeedFacts` aggregate in
one value operation instead of copying synchronized fields individually; an
event-based test pins snapshot facts to reducer navigation facts.
Tool-card output summaries now use one normalized `(call_id, member_index)`
identity index while retaining first-seen transcript order, removing the
quadratic derived-card lookup.
The activity counter projection now reads the normalized facts tuple directly,
removing an obsolete private forwarding layer.

`RemoveKind` now prunes tool facts and row-keyed display modes against the live
transcript identity set. A reducer regression covers orphan removal while
retaining a live tool card, closing the stale-derived-index failure mode.

Acceptance: snapshot parity, replay parity, and no stale derived index after
insert/update/remove sequences. The lifecycle, interleaving, YAML, and
`RemoveKind` orphan-pruning regressions provide that evidence. This task is
adopted.
