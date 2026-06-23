# Add per-channel stream decoders for UI projections

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: r5-populate-parts-streaming
**Blocks**: none

## Description

Currently all `Event`s flow through a single flat `EventBus` to the `UiActor`, which applies every event to `AppState` and publishes a full `Snapshot`. As the UI grows more complex (splitpanes, tool-call detail panels, thinking viewer, subagent sidebars), every consumer gets every event even if it only cares about a subset. LangGraph solves this with per-channel state-machine decoders (`libs/sdk-py/langgraph_sdk/stream/decoders.py:1-359`): a `MessagesDecoder` tracks active message streams, a `ToolCallsDecoder` tracks active tool-call handles, each projecting only the events they care about. Add a `ChannelDecoder` trait and three concrete decoders (`TextChannel`, `ToolCallChannel`, `ReasoningChannel`) that subscribe to relevant `Event` variants and maintain their own state, so UI panels can query a single channel instead of scanning the full `AppState`.

## Acceptance Criteria

- [ ] New module `crates/runie-core/src/channels.rs` declares:
  - `pub trait ChannelDecoder { type Output; fn process(&mut self, event: &Event) -> Option<&Self::Output>; }`.
  - `pub struct TextChannel { current: Option<String>, finished: Vec<String> }` — processes `ResponseDelta` (appends to `current`), `Response` (pushes to `finished`), `TurnComplete` (flushes `current` to `finished`). `output()` returns `&[String]` of finished text blocks.
  - `pub struct ToolCallChannel { active: HashMap<String, ToolCallState>, completed: Vec<ToolCallResult> }` — processes `ToolStart` (inserts into `active`), `ToolEnd` (moves to `completed`). `output()` returns completed tool calls.
  - `pub struct ReasoningChannel { current: Option<String>, finished: Vec<String> }` — processes `ThoughtDone` (pushes to `finished`), `Thinking` (starts `current`).
- [ ] `EventBus` gains a `subscribe_channel<C: ChannelDecoder>(&self, decoder: C) -> mpsc::Receiver<C::Output>` method that filters events through the decoder and sends `Output` items to a channel.
- [ ] `UiActor` uses a `TextChannel` for the message feed, a `ToolCallChannel` for the tool-call sidebar (if shown), and a `ReasoningChannel` for the thinking panel (if expanded). Each panel reads from its channel receiver instead of scanning `AppState`.
- [ ] The full `Snapshot` path remains for backward compat — channels are an opt-in optimization.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `text_channel_accumulates_deltas` — feed `[ResponseDelta("hi"), ResponseDelta(" there")]`; `current` is `Some("hi there")`, `finished` is empty.
- [ ] `text_channel_flushes_on_turn_complete` — feed `[ResponseDelta("hi"), TurnComplete]`; `finished` is `["hi"]`, `current` is `None`.
- [ ] `tool_call_channel_tracks_active_and_completed` — feed `[ToolStart(id="t1", name="bash"), ToolEnd(id="t1", output="done")]`; `active` is empty, `completed` has one entry.
- [ ] `reasoning_channel_collects_thoughts` — feed `[Thinking, ThoughtDone]`; `finished` has one entry.
- [ ] `channel_ignores_irrelevant_events` — `TextChannel::process(ToolStart {...})` returns `None` and doesn't mutate state.

### Layer 2 — Event Handling
- [ ] `event_bus_subscribe_channel_filters_events` — subscribe a `TextChannel`, emit `[ToolStart, ResponseDelta("hi"), ToolEnd, TurnComplete]` on the bus; the channel receiver receives only the text output.
- [ ] `ui_actor_text_channel_matches_snapshot` — run a turn, compare `TextChannel::output()` to the text in `Snapshot.session.messages`; they match.

### Layer 3 — Rendering
- [ ] `render_text_channel_output` — a panel rendered from `TextChannel::output()` shows the same text as the current `AppState`-based render.

### Layer 4 — Smoke / Crash
- [ ] `smoke_channels_module_present` — `ls crates/runie-core/src/channels.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-core/src/channels.rs` (new, ~150 LOC)
- `crates/runie-core/src/lib.rs` (add `pub mod channels;`)
- `crates/runie-core/src/event_bus.rs` (add `subscribe_channel`, ~30 LOC)
- `crates/runie-tui/src/ui_actor.rs` (opt-in channel usage for panels, ~20 LOC)

## Notes

Source inspiration: LangGraph `libs/sdk-py/langgraph_sdk/stream/decoders.py:1-359` (per-channel state machines: `MessagesDecoder`, `ToolCallsDecoder`, `SubgraphsDecoder`) and `libs/langgraph/langgraph/stream/stream_channel.py:14-341` (`StreamChannel` single-consumer drainable queue). The Rust version is simpler because we don't need LangGraph's mini-mux hierarchy or `tee` fan-out — `mpsc::Receiver` is sufficient for single-consumer per-panel. This is a P2 task because the current single-snapshot path works; channels are an optimization for when the UI grows more panels. Depends on `r5-populate-parts-streaming` because the decoders need structured `Part` data to project cleanly (e.g., `TextChannel` reads `Part::Text` boundaries, not monolithic `content`).
