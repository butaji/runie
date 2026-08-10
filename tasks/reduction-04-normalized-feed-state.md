# Reduction 04: normalized feed state

Status: partial

Store canonical feed records and derive tool/activity/navigation projections;
remove synchronized duplicate indexes where possible.

Progress: the feed actor now retains the reduced event sequence through
`EventMemo`; normalized records and derived-index removal remain next.

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
clear/reset reducers no longer duplicate field-by-field projection logic.
The full facts clear operation is also centralized on `FeedFacts`, leaving the
feed reducer's clear path responsible only for transcript and navigation data.

`ToolRecord` now owns name/argument mutation operations, so reducer stages no
longer duplicate record construction and argument clearing semantics.

Remaining work is removal of duplicated derived indexes and normalization of
individual line/tool records.

Acceptance: snapshot parity, replay parity, and no stale derived index after
insert/update/remove sequences.
