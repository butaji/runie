# R5 Populate Parts Streaming

**Status**: done
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: r5-per-channel-decoders

## Description

Implement streaming of partial parts (tool call fragments, thinking fragments) to the UI as they arrive, not just when complete. This enables smoother UX during long tool calls.

## Implementation

Added `ToolInputDelta` event to stream partial tool call input arguments to the UI:

1. **New event variant** (`crates/runie-core/src/event/variants.rs`):
   - Added `ToolInputDelta { id: String, content: String }` for partial tool call argument deltas

2. **Provider event conversion** (`crates/runie-core/src/event/from_provider_event.rs`):
   - Updated `tool_input_delta()` to map `ProviderEvent::ToolCallInputDelta` → `Event::ToolInputDelta`
   - Previously incorrectly mapped to `ResponseDelta`

3. **Event taxonomy** (`crates/runie-core/src/event/kind/mod.rs`):
   - Added `ToolInputDelta` to `is_llm_agent_fact()` - it's a fact about streaming state

4. **Tests**:
   - Added `tool_input_delta_maps_to_tool_input_delta_event` test in `from_provider_event::tests`
   - Updated `dispatcher_handles_all_variants` to include `ToolInputDelta`

## Acceptance Criteria

- [x] Partial tool calls streamed to UI (`ToolInputDelta` event added)
- [x] Partial thinking streamed to UI (already existed as `ThinkingDelta`)
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `tool_input_delta_maps_to_tool_input_delta_event` — verifies correct event mapping

### Layer 2 — Event Handling
- [x] `dispatcher_handles_all_variants` — exhaustive match covers `ToolInputDelta`

### Layer 3 — Rendering
- [x] N/A — event-only change, rendering follows from existing partial streaming

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Covered by existing provider event replay tests

## Files touched

- `crates/runie-core/src/event/variants.rs` — added `ToolInputDelta` variant
- `crates/runie-core/src/event/from_provider_event.rs` — updated conversion
- `crates/runie-core/src/event/kind/mod.rs` — added to `is_llm_agent_fact()`
- `crates/runie-core/src/event/variants_tests/dispatch.rs` — updated exhaustive test

## Notes

- R5 milestone task
- Enables better UX during tool calls by streaming partial arguments
- The `ToolStream` state machine in `tool_stream.rs` already accumulates partial JSON; this event makes those deltas visible to the UI
