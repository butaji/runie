# p47 — Migrate renderer transient state into actor reducers

Status: in progress — live tool/assistant/turn ownership migrated; compatibility cleanup remains (2026-08-07)

## Production call-site audit (2026-08-07)

An exhaustive search of `EventRenderer` mutation call sites confirms that the
remaining `Arc<Mutex<Scrollback>>`/`Arc<Mutex<StatusBar>>` writes occur only in
the compatibility projection branches (`scrollback_actor.is_none()` or
`status_actor.is_none()`) and in synchronous renderer tests. Actor-backed
production construction uses acknowledged `ScrollbackMsg`/`StatusMsg`
delivery and reads snapshots; it does not mutate sibling actor state through
the renderer.

This is an architecture evidence checkpoint, not closure: the compatibility
adapter still exists and must be retired after its callers are migrated. The
source boundary validator continues to enforce the live feed ownership rule.

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
| `pending_tools` | compatibility header/update/argument accumulation keyed by tool ID | `FeedState` live tool block/header |
| `in_assistant_stream` | compatibility stream lifecycle | `FeedState` / core projection (live migrated) |
| — | reasoning/assistant text body | `FeedState` reasoning/assistant lines (compatibility accumulator removed) |
| — | thinking duration | `StatusState`/core assistant terminal event (compatibility cache removed) |
| activity counters and `activity_group_open` | compatibility activity aggregation | `ScrollbackActor` feed activity aggregate (live migrated) |
| — | pending-tool count/lifecycle flag | pending tool identity map / feed reducer (compatibility duplicates removed) |
| `turn_started` | whether terminal summary is due | `FeedState` / `FeedSnapshot` (migrated) |

The legacy synchronous constructors may retain a compatibility adapter during
the migration, but production must have one owner for each field. A field is
not considered migrated merely because its final `Line` is sent through a
mailbox. The live `App` now satisfies this boundary; remaining legacy widget
mutexes are confined to synchronous replay/test constructors.

## Migration order

1. Add serializable, renderer-independent pending-tool/activity facts to
   `FeedSnapshot` and pure reducer messages for start/update/end transitions.
2. Move the remaining compatibility header/argument accumulation into
   `FeedState`; make the event renderer's tool handlers pure message
   constructors.
3. Move the remaining assistant lifecycle guard into the feed reducer while
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

Assistant stream lifecycle increment (2026-08-07): `FeedState` now owns the
`assistant_stream_open` fact. `AssistantStreamStart` and `AssistantStreamEnd`
are explicit reducer messages emitted by the event-to-feed mapping; the
compatibility adapter applies the same messages to its model. Delta handling
therefore reads one actor snapshot predicate instead of a renderer boolean,
and reset clears the lifecycle fact. A pure reducer test covers the complete
start/end/clear sequence.

Tool-row identity increment (2026-08-07): removed the compatibility
`tool_rows` index from `EventRenderer`. The feed model already carries each
row's opaque tool-call identity and active/settled state; compatibility
updates now resolve the newest matching row from that projection at the
mutation boundary. Parallel tool updates remain ID-based, while the
renderer no longer owns a second mutable row-index map.

Pending-tool ownership increment (2026-08-07): removed the compatibility
`pending_tools` header/argument map. Tool arguments now cross the event
boundary through `SetToolArgs`/`RemoveToolArgs` and live in the feed snapshot;
compatibility updates derive the current header and arguments from that
projection. This preserves parallel tool identity and terminal header
formatting while eliminating the renderer's second pending-lifecycle store.

Activity-group ownership increment (2026-08-07): removed the compatibility
`activity_group_open` boolean. Group continuity is now derived from the
actor/compatibility feed projection: an activity row after the newest user
entry identifies the current Grok activity group. Starting a new user turn
therefore naturally starts a fresh group without a renderer reset flag.

Activity-counter ownership increment (2026-08-07): activity counts now live in
`FeedNavigation`/`FeedSnapshot` and change only through
`ActivityReset`/`ActivityToolStart`/`ActivityToolEnd` messages. The renderer no
longer owns directory/file/command/subagent/failure counters; activity labels
are formatted from actor facts, including the live and compatibility paths.

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

Dead-state cleanup (2026-08-07): removed the compatibility `in_reasoning`
flag, which was assigned on thinking/text transitions but never read. The
reasoning buffer remains the sole compatibility text accumulator; live
reasoning continues to come from the feed actor snapshot.

Assistant text ownership increment (2026-08-07): removed the compatibility
`streaming_buffer`. Text deltas now append directly to the existing assistant
line in the compatibility feed model, and finalization reads that model. The
live path was already feed-actor-owned; this removes the duplicate text
accumulator from replay without changing Grok line ordering.

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

Reasoning text ownership increment (2026-08-07): removed the compatibility
`reasoning_buffer`. Thinking deltas now append directly to the feed's reasoning
line, and assistant finalization derives `has_reasoning` from feed lines before
applying Grok's collapsed/expanded projection. The reasoning visual snapshot
and YAML replay remain green.

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

## Compatibility-state audit (2026-08-07)

The live `EventRenderer::with_live_actors` path was re-audited after the
animation/event work. Its mutable-looking helpers derive from actor snapshots
and emit acknowledged `ScrollbackMsg`/`StatusMsg` messages; the legacy
`Projection<Scrollback>` and `Projection<StatusBar>` variants are compiled only
for synchronous compatibility tests. No live renderer field remains a second
state owner. The remaining cleanup is test/replay adapter retirement, not a
production SSOT violation.

The same audit compared dense-group truncation with Grok's renderer source.
Runie's plain `N more` fallback deliberately uses `hidden - 1`, because Grok's
synthetic header consumes the first hidden slot; this is source-aligned and is
covered by the dense-group YAML oracle. The next TUI-only gap is Grok's
mouse/text-selection box and per-member selection surface, which requires an
explicit interaction event contract before implementation.

## Selection-range event slice (2026-08-07)

`FeedState` now owns `selection_anchor` and `selection_head`, with
`ScrollbackMsg::SelectRange`/`ClearSelection` as the only transition inputs.
The YAML `select_range` event and `visual-selection-range.yaml` fixture verify
the values after the real actor replay and after model-to-widget snapshot
rehydration. This is the state boundary needed before mapping crossterm mouse
coordinates; cell-range painting and clipboard actions remain separate
renderer/input work and are not claimed complete here.

Keyboard intent slice (2026-08-07): Shift+Up/Down now maps to explicit
`ExtendSelectionPrevious`/`ExtendSelectionNext` actions when the prompt is
empty. The application converts that intent into an acknowledged
`SelectRange` message; typed prompt editing remains isolated from transcript
selection. Mouse coordinate mapping and clipboard integration remain open.

## Mouse wheel event slice (2026-08-07)

The interactive input actor now accepts crossterm mouse events instead of
discarding them. `ScrollUp` and `ScrollDown` are reduced to fixed ±3-line
scroll intents and delivered through `App::scroll_scrollback_by`, which sends
the acknowledged `ScrollbackMsg::ScrollBy` to the feed actor. Other mouse
events remain inert until Grok-compatible coordinate selection and clipboard
contracts are modeled. A focused binary test pins the pure wheel mapping.

## Grok scroll-normalization acceptance contract (2026-08-07)

Source audit of `grok-build/src/input/mouse.rs` records the remaining exact
behavior required before this slice can be marked parity-complete:

- default wheel throughput is 3 lines per tick, with terminal-specific event-
  per-tick profiles and optional `scroll_lines`/speed/inversion overrides;
- events are grouped into streams with an 80 ms gap and flushed on a 16 ms
  redraw cadence;
- auto mode distinguishes wheel from trackpad using event density and timing,
  retains fractional trackpad accumulation, and applies bounded acceleration;
- direction, carry, duplicate terminal reports, stream caps, and viewport
  bounds are normalized before the actor receives the final line delta.

Runie's current ±3 mapping intentionally covers only the default wheel case;
the normalizer and its YAML/event-sequence oracle remain open.

Mouse capture lifecycle closure (2026-08-07): the interactive terminal now
enables crossterm mouse capture together with the alternate screen and disables
it during restoration. This makes the already-owned mouse event path reachable
in real terminals; it does not claim the normalization contract above.
