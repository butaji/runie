# p47 — Migrate renderer transient state into actor reducers

Status: in progress — live tool/assistant/turn ownership migrated; compatibility cleanup remains (2026-08-07)

## Why this task exists

The production `EventRenderer` no longer owns the durable feed snapshot: it
delivers `ScrollbackMsg` values to `ScrollbackActor`, and rendering reads the
actor snapshot. However, the renderer still keeps event-reduction metadata
between events. That metadata is state and must ultimately belong to the feed
actor if the SSOT rule is to be literal.

Source: `crates/runie-tui/src/event_renderer.rs`, `EventRenderer` fields and
`handle_tool_*` / `handle_message_*` methods. The corresponding owner is
`crates/runie-tui/src/scrollback_actor.rs` and its pure `FeedState` reducer in
`crates/runie-tui-model/src/feed.rs`.

## Field inventory and target owner

| Current renderer field | State represented | Target actor-owned projection |
|---|---|---|
| `tool_rows` | compatibility row identity | `FeedState` tool row identity (already modeled by `tool_row_id`) |
| `tool_buffers` | compatibility header/update accumulation | `FeedState` live tool block/header |
| `tool_args` | compatibility tool-card metadata | `FeedState` pending tool metadata |
| `in_assistant_stream` | compatibility stream lifecycle | `FeedState` / core projection (live migrated) |
| `in_reasoning`, `reasoning_buffer` | compatibility thinking body | `FeedState` reasoning projection (live migrated) |
| `thinking_elapsed_ms` | compatibility test fallback | `StatusState`/core assistant terminal event (live migrated) |
| activity counters and `activity_group_open` | compatibility activity aggregation | `ScrollbackActor` feed activity aggregate (live migrated) |
| `active_tool_count`, `in_tool_exec` | compatibility pending-tool lifecycle | core state pending-tool projection / feed reducer (live migrated) |
| `turn_started` | whether terminal summary is due | `FeedState` / `FeedSnapshot` (migrated) |

The legacy synchronous constructors may retain a compatibility adapter during
the migration, but production must have one owner for each field. A field is
not considered migrated merely because its final `Line` is sent through a
mailbox. The live `App` now satisfies this boundary; remaining legacy widget
mutexes are confined to synchronous replay/test constructors.

## Migration order

1. Add serializable, renderer-independent pending-tool/activity facts to
   `FeedSnapshot` and pure reducer messages for start/update/end transitions.
2. Move header accumulation and argument retention into `FeedState`; make the
   event renderer's tool handlers pure message constructors.
3. Move reasoning/assistant stream accumulation into the feed reducer while
   preserving Pi event ordering and Grok collapsed/expanded projections.
4. Derive activity summaries and turn-summary eligibility from actor state;
   remove the renderer counters and lifecycle booleans.
5. Delete the production `Projection<Scrollback>` compatibility branch. Keep
   only test/replay adapters until all callers use actor snapshots.

### First increment complete (2026-08-06)

Thinking duration now follows `ThinkingEnd`/assistant terminal events into the
actor-owned `StatusSnapshot`. Production finalization reads that snapshot for
the Grok `Thought for …` label; only the compatibility test renderer retains a
test-scoped fallback. `visual-thinking-duration.yaml` asserts the event-to-
snapshot path directly, so changing the event sequence does not require
recompilation.

Turn lifecycle is also now an actor-owned feed fact: `TurnStart` and `TurnEnd`
are explicit reducer messages and `FeedSnapshot.turn_started` determines
whether the production path emits Grok's terminal `Worked for` row. The
renderer no longer uses its lifecycle flag for production summary eligibility;
the compatibility reducer remains available for synchronous tests.
The baseline `visual-hey.yaml` fixture now asserts the final `false` value
directly from the feed snapshot.

The live metadata bridge no longer writes a renderer `turn_started` field. The
compatibility path also reduces `TurnStart`/`TurnEnd` through the `Scrollback`
model, so the renderer has no duplicate turn-lifecycle field.

Turn lifecycle ownership increment (2026-08-07): the compatibility renderer no
longer keeps a duplicate `turn_started` field. `TurnStart`/`TurnEnd` reduce the
compatibility `Scrollback` model through `ScrollbackMsg`, and finalization
reads the same model snapshot used by the actor path. This removes one
renderer-side lifecycle mirror while preserving replay behavior.

Thinking-duration ownership increment (2026-08-07): the compatibility
`StatusBar` now retains `thinking_elapsed_ms` in its renderer-independent
snapshot, exactly like `StatusState`. `EventRenderer` no longer keeps a
test-only duration cache or records terminal thinking events itself; the
existing `StatusMsg::SetThinkingElapsed` projection is authoritative for both
live and synchronous replay paths. This removes one renderer-side mirror
without changing the YAML `visual-thinking-duration` contract.

Tool lifecycle cleanup increment (2026-08-07): removed the compatibility
`in_tool_exec` boolean. It could become false when one of several parallel
tool calls completed, causing updates for still-running siblings to be
dropped. Compatibility updates now use the owned pending-tool count plus the
tool-id buffer as their event-derived predicate, matching the parallel-tool
identity contract already covered by the renderer tests. The live path remains
fully actor-owned.

Pending-tool ownership increment (2026-08-07): the compatibility
`active_tool_count` counter is now derived from the pending tool map
keyed by tool-call ID. This removes another mutable count that could diverge
from lifecycle events; start/update/end behavior remains identity-based and
parallel-tool coverage passes.

Pending-tool map increment (2026-08-07): the compatibility header and argument
maps are now one typed `PendingTool` map keyed by tool-call ID. This makes the
identity, mutable header, and completion arguments one reducer-local value;
there is no opportunity for the two former maps to diverge.

Assistant metadata ownership increment (2026-08-07): the live
`with_live_actors` path no longer runs `handle_message_start/update/end` through
the compatibility metadata reducer. Assistant text/reasoning lifecycle is
already reduced by `ScrollbackActor`; the renderer retains those buffers only
for synchronous compatibility/replay adapters. A regression asserts that live
assistant events leave the compatibility buffers empty.

Reset invariant (2026-08-07): `FeedState::clear` now resets its actor-owned
`turn_started` fact. This prevents a reset event from leaking a stale
completion-summary eligibility bit into a subsequent replay or live session.

## Tool lifecycle increment (2026-08-06)

The live `App` now constructs `ScrollbackActor::new_with_bus`. Its owned bus
projection is the production reducer for tool headers, arguments, activity
counters, structured updates, completion output, and errors. The live
`EventRenderer` no longer runs its duplicate tool lifecycle reducer;
`with_actors` remains the deterministic replay/compatibility adapter. This
removes a second production source of tool-card state while preserving the
same event vocabulary and YAML replay path.

## Assistant transcript increment (2026-08-06)

Live assistant deltas are already reduced by `ScrollbackActor` from the
canonical `MessageStart`/`MessageUpdate` events. Live finalization now derives
reasoning presence, fold policy, and pending-tool facts from that actor
snapshot; the renderer no longer uses its compatibility streaming/reasoning
buffers for the live path. The synchronous `with_actors` replay adapter keeps
those buffers for its existing deterministic compatibility contract.

## Acceptance evidence

- Every migrated field is present in `FeedSnapshot` or a core actor snapshot;
  no production `EventRenderer` field stores it.
- YAML fixtures replay the same start/update/end event sequence and assert the
  actor state before checking the screen.
- No sleeps are added; all acknowledgements settle after reducer application.
- `just ci`, the full YAML suite, and the four-size full-cell visual matrix
  remain green.
- A source audit shows no production path mutates another actor or reconstructs
  a pending tool from rendered text.

This task is intentionally separate from the strict color oracle gap in p19/p25:
state ownership can be completed independently, while color parity requires a
clean paired Grok capture.
