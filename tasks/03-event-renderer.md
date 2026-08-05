# Step 03: EventRenderer

**Status:** pending
**Depends on:** 02

## Goal
A translator from `runie_core::AgentEvent` → mutation calls on widgets. Owns the live streaming line buffer.

## Changes
- `crates/runie-tui/src/event_renderer.rs`:
  - `EventRenderer` struct with mutable references to scrollback + prompt + status widgets.
  - `pub async fn run(self, mut bus_rx: broadcast::Receiver<AgentEvent>)`.
  - `apply_event(&mut self, event: AgentEvent)`: dispatches each event variant to widget mutations per the table in the plan file.
  - Live streaming buffer: while in a streaming assistant message, accumulated text deltas live in `streaming_buffer: String`. `MessageEnd(assistant)` flushes the buffer into the scrollback as a finalized line.

## Verification
- Unit test: feed a sequence of events into `apply_event` and assert the scrollback's line count matches the expected transcript.

## Notes
- Pure mutation, no IO. The async `run` method only awaits the bus receiver.
- Each `apply_event` is sync (no awaits) so the renderer can drain many events without yielding.