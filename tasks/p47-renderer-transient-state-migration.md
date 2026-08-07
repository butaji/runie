# p47 — Migrate renderer transient state into actor reducers

Status: planned / next architecture increment (2026-08-06)

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
| `tool_rows` | live row identity for compatibility updates | `FeedState` tool row identity (already modeled by `tool_row_id`) |
| `tool_buffers` | accumulated tool header/update text | `FeedState` live tool block/header |
| `tool_args` | arguments needed to settle a tool card | `FeedState` pending tool metadata |
| `in_assistant_stream` | assistant stream lifecycle | `FeedState` or core `AgentStateSnapshot` projection |
| `in_reasoning`, `reasoning_buffer` | thinking section lifecycle/body | `FeedState` reasoning projection |
| `thinking_elapsed_ms` | terminal thinking duration | `StatusState`/core assistant terminal event |
| activity counters and `activity_group_open` | Grok activity-group aggregation | `FeedState` activity aggregate |
| `active_tool_count`, `in_tool_exec` | pending tool lifecycle | core state pending-tool projection / feed reducer |
| `turn_started` | whether terminal summary is due | core lifecycle state or feed reducer |

The legacy synchronous constructors may retain a compatibility adapter during
the migration, but production must have one owner for each field. A field is
not considered migrated merely because its final `Line` is sent through a
mailbox.

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
