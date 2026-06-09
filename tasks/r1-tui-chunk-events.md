# Streaming: event per chunk

**Status**: done
**Milestone**: R1
**Category**: TUI Improvements

## Description

Each LLM chunk emitted as individual event for streaming UI updates.

## Acceptance Criteria

- [x] ResponseChunk event per chunk — `run_agent_turn()` emits `Event::AgentResponse` for each provider chunk
- [x] Event loop handles each chunk — `update/mod.rs` calls `append_response(id, content)` per `AgentResponse`
- [x] No buffering — chunks flow directly from provider → event loop → state update

## Implementation

`crates/runie-agent/src/turn.rs`:
```rust
provider.generate(messages.clone(), |chunk| {
    response_text.push_str(&chunk.content);
    emit(Event::AgentResponse {
        id: command.id.clone(),
        content: chunk.content,
    });
}).await?;
```

Each provider chunk triggers a state update via the event loop.

## Tests

- [x] Layer 1 — `tests/element_order.rs`, `tests/flow.rs` verify chunk ordering
- [x] Layer 2 — `runie-agent/src/turn.rs` emits `AgentResponse` per chunk
- [x] Layer 3 — Streaming UI renders each chunk incrementally
- [x] Layer 4 — End-to-end streaming verified

## Notes

- Streaming works correctly. Each chunk updates state and triggers a re-render.
- The render task drops old snapshots if behind, so the UI always shows the latest state.
