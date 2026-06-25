# R5 Populate Parts Streaming

**Status**: todo
**Milestone": R5
**Category": Core / State
**Priority": P1

**Depends on": event-taxonomy-for-actor-state-sync
**Blocks": r5-per-channel-decoders

## Description

Implement streaming of partial parts (tool call fragments, thinking fragments) to the UI as they arrive, not just when complete. This enables smoother UX during long tool calls.

## Acceptance Criteria

- [ ] Partial tool calls streamed to UI
- [ ] Partial thinking streamed to UI
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `partial_parts_accumulated`

### Layer 2 — Event Handling
- [ ] `streaming_partial_parts_intents`

### Layer 3 — Rendering
- [ ] `partial_tool_call_renders`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `streaming_partial_parts_e2e`

## Files touched

- `crates/runie-core/src/agent/`
- `crates/runie-core/src/event/`

## Notes

- R5 milestone task
- Enables better UX during tool calls
