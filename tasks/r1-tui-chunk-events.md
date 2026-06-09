# Streaming: event per chunk

**Status**: done

**Milestone**: R1

**Category**: TUI Improvements

## Description

Each LLM chunk emitted as individual event for streaming UI updates.

## Acceptance Criteria

- [x] ResponseChunk event per chunk
- [x] ChatAgent accumulates chunks
- [x] No buffering in Orchestrator

## Tests

- [x] Layer 1 — State/logic: `tests/element_order.rs`, `tests/flow.rs` verify chunk ordering
- [x] Layer 2 — Event handling: `runie-agent` emits AgentResponse per chunk
- [x] Layer 3 — Rendering: Streaming UI renders each chunk incrementally
- [x] Layer 4 — Smoke: End-to-end streaming verified via manual testing
