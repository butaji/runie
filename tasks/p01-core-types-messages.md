# p01 — Core types: AssistantMessage / ToolResultMessage / UserMessage field parity

**Parity target:** pi-agent-core message types.

## Pi reference

- `AssistantMessage` — `~/Code/agents/pi/packages/ai/src/types.ts:399`
  ```ts
  { role:"assistant"; content: (TextContent|ThinkingContent|ToolCall)[];
    api: Api; provider: ProviderId; model: string;
    responseModel?; responseId?; diagnostics?; usage: Usage;
    stopReason: StopReason; errorMessage?; rawStopReason?; timestamp: number }
  ```
- `ToolResultMessage` — `types.ts:415`
  ```ts
  { role:"toolResult"; toolCallId; toolName;
    content: (TextContent|ImageContent)[];
    details?; usage?; addedToolNames?: string[]; isError: boolean; timestamp }
  ```
- `UserMessage` — `types.ts:393`: `{ role:"user"; content: string | (TextContent|ImageContent)[]; timestamp }`. Note pi allows a **plain string** content (not just an array).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/types.rs`
- `AssistantMessage` (types.rs:115): only `content`, `stop_reason`, `model`, `timestamp`. **Missing:** `usage`, `error_message`, `raw_stop_reason`, `api`, `provider`, `response_model`, `response_id`, `diagnostics`.
- `ToolResultMessage` (types.rs:133): `tool_call_id`, `tool_name`, `content`, `is_error`, `timestamp`. **Missing:** `details`, `usage`, `added_tool_names`.
- `UserMessage` (types.rs:91): only `Vec<UserContent>` (no plain-string form).

## Adapt to runie

Add the missing fields with `#[serde(default)]` where pi marks them optional:

```rust
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub stop_reason: Option<StopReason>,
    pub model: String,
    pub api: String,            // pi: api
    pub provider: String,       // pi: provider
    pub usage: Usage,           // pi: usage (required)
    pub error_message: Option<String>,   // pi: errorMessage?
    pub raw_stop_reason: Option<String>, // pi: rawStopReason?
    pub timestamp: i64,
}
```

```rust
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ToolResultContent>,
    pub details: serde_json::Value,   // pi: details?
    pub usage: Option<Usage>,          // pi: usage?
    pub added_tool_names: Vec<String>, // pi: addedToolNames?
    pub is_error: bool,
    pub timestamp: i64,
}
```

`UserMessage`: keep `Vec<UserContent>` (Rust has no string-or-array union without extra enum); add a constructor `UserMessage::text(s, ts)` and a `WireMessage` conversion that accepts a plain string. Document the deviation (pi's `string` is sugar for `[{type:"text",text}]`).

## State machine / variants

- These are **data structures**, not state machines. The relevant "variants" are the content-block unions:
  - `AssistantContent` (types.rs:107): `Text | Thinking | ToolCall` — already matches pi `TextContent|ThinkingContent|ToolCall`.
  - `UserContent` (types.rs:84): `Text | Image` — matches pi.
  - `ToolResultContent` (types.rs:125): `Text | Image` — matches pi.
- Ensure serde `role`/`type` tags match pi exactly (`"assistant"`, `"toolResult"`, snake_case `"tool_call"` id, `"thinking"` block).

## Acceptance

- `cargo test -p runie-core` green; new round-trip serde tests fill every new field and assert exact JSON keys match pi's TS shapes.
- `wire_to_agent` / `default_convert_to_llm` (convert.rs) preserve the new fields.
- Provider `replay.rs` populates `usage`/`raw_stop_reason` from SSE `done` payloads.