# Unify tool-call accumulators into a single state machine

## Status

`todo`

## Context

Runie has three overlapping tool-call accumulators: `ToolStream` in `crates/runie-core/src/tool_stream.rs`, `ToolAccum` in `crates/runie-provider/src/openai/protocol.rs`, and `ToolAccumulator`/`ToolRegistry` in `crates/runie-provider/src/protocol.rs`. They must be kept in sync.

## Goal

Collapse the three accumulators into one provider-agnostic state machine owned by the streaming response parser. Provider crates emit normalized deltas; the single accumulator builds complete tool calls.

## Acceptance Criteria

- [ ] Delete `ToolAccum` and `ToolAccumulator` duplicates.
- [ ] Move accumulation logic to `runie_core::tool_stream::ToolStream`.
- [ ] OpenAI protocol emits only normalized `ToolCallDelta` events.
- [ ] All streaming tool-call tests pass.

## Design Impact

No change to TUI element design or composition. Only internal tool-call streaming behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for partial, interleaved, and complete tool-call deltas.
- **Layer 2 — Event Handling:** `AgentEvent::ToolCallStarted`/`Updated`/`Done` sequence is unchanged.
- **Layer 3 — Rendering:** `TestBackend` shows tool-call progress identically.
- **Layer 4 — E2E:** Provider replay fixture with multi-tool streaming passes.
- **Live tmux validation:** Start a turn that calls multiple tools; tool cards render correctly and complete in order.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
