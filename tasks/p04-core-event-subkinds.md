# p04 — Core types: granular `AssistantMessageEvent` sub-kinds

**Parity target:** pi streaming event sub-kinds.

## Pi reference

`AssistantMessageEvent` (pi-ai `~/Code/agents/pi/packages/ai/src/types.ts:501`) is a discriminated union whose `type` field has **13** members:
- `start`
- `text_start` / `text_delta` / `text_end` (carry `contentIndex`, `delta`/`content`, `partial`)
- `thinking_start` / `thinking_delta` / `thinking_end`
- `toolcall_start` / `toolcall_delta` / `toolcall_end` (carry `contentIndex`, `partial`/`toolCall`)
- `done` (`reason: "stop"|"length"|"toolUse"`, `message`)
- `error` (`reason: "aborted"|"error"`, `error`)

The loop only synthesizes `message_update` for the **delta** sub-kinds (`text_delta`, `thinking_delta`, `toolcall_delta`); `start` → `message_start`, and `done`/`error` → `message_end` (`agent-loop.ts:319-371`).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/types.rs:413`
```rust
pub enum AssistantMessageEvent {
    Start, TextDelta{delta}, ThinkingDelta{delta},
    ToolCallDelta{index,partial}, Done{stop_reason,usage}, Error{error}
}
```
6 variants; missing the sectional `start`/`end` and `toolcall_end` markers.

## Adapt to runie

Add the fine-grained markers so downstream renderers (reasoning folding, text/tool boundaries) can match pi exactly:

```rust
pub enum AssistantMessageEvent {
    Start,
    TextStart { index: usize },
    TextDelta { index: usize, delta: String },
    TextEnd { index: usize },
    ThinkingStart { index: usize },
    ThinkingDelta { index: usize, delta: String },
    ThinkingEnd { index: usize },
    ToolCallStart { index: usize, partial: ToolCall },
    ToolCallDelta { index: usize, partial: ToolCall },
    ToolCallEnd { index: usize, tool_call: ToolCall },
    Done { stop_reason: StopReason, usage: Usage },
    Error { error: String },
}
```
Keep serialization keys matching pi (`text_start`, `text_delta`, `toolcall_end`, etc.).

## State machine / variants

The **streaming state machine** per content index:
```
start → (text|thinking|toolcall)* → done | error
        text_start → text_delta* → text_end
        thinking_start → thinking_delta* → thinking_end
        toolcall_start → toolcall_delta* → toolcall_end
```
- `ToolCallDelta` accumulates `partial.arguments` via streaming-JSON parse (see proxy `toolcall_delta`, `~/Code/agents/pi/packages/agent/src/proxy.ts:310`, `parseStreamingJson`); `toolcall_end` finalizes and drops the partial JSON.
- `apply_event` (driver.rs) must map `TextStart`/`TextEnd` etc. onto content blocks without double-append (reuse `push_or_append`).

## Acceptance

- Round-trip serde test over all 13 wire forms.
- `message_update` is emitted **only** for `*_delta`; `start`→`message_start`, `done`/`error`→`message_end` (matching `agent-loop.ts:319-371`).
- runie-tui renderer consumes the new markers (reasoning fold, text/tool sections) — see p16.
- `cargo test --workspace` green.