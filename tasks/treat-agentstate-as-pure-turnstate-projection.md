# Treat `AgentState` as pure `TurnState` projection

## Status

`done`

## Description

`AgentState` is fully derived from `TurnState` via `From<&TurnState>`. All fields (queues, lifecycle, tool execution, tokens, speed, streaming, IDs) are mirrored from `TurnState` using the `From` impl. No fields are updated independently.

## Implementation

```rust
impl From<&TurnState> for AgentState {
    fn from(ts: &TurnState) -> Self {
        Self {
            request_queue: ts.request_queue.clone(),
            message_queue: ts.message_queue.clone(),
            current_request_id: ts.current_request_id.clone(),
            turn_started_at: ts.turn_started_at,
            turn_active: ts.turn_active,
            inflight: ts.inflight,
            current_tool_name: ts.current_tool_name.clone(),
            tool_started_at: ts.tool_started_at,
            intermediate_step_count: ts.intermediate_step_count,
            tokens_in: ts.tokens_in,
            tokens_out: ts.tokens_out,
            turn_tokens_out: ts.turn_tokens_out,
            token_tracker: ts.token_tracker.clone(),
            speed_tps: ts.speed_tps,
            speed_window: ts.speed_window.clone(),
            last_speed_update: ts.last_speed_update,
            tokens_at_last_speed: ts.tokens_at_last_speed,
            streaming: ts.streaming,
            streaming_buffer: ts.streaming_buffer.clone(),
            next_id: ts.next_id,
            current_action: ts.current_action.clone(),
            thought_seq: ts.thought_seq,
            last_assistant_index: ts.last_assistant_index,
            thinking_started_at: ts.thinking_started_at,
            tokens_in_display: ts.tokens_in_display,
            tokens_out_display: ts.tokens_out_display,
            tokens_in_prev: ts.tokens_in_prev,
            tokens_out_prev: ts.tokens_out_prev,
        }
    }
}
```

## Acceptance criteria

1. ✅ **Unit tests** — `AgentState::from(&turn_state)` is used for all projections.
2. ✅ **E2E tests** — Replay produces identical `AgentState`.
3. ✅ **Live tmux tests** — Run a turn and verify UI state matches turn state.
