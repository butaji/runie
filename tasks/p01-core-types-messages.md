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

## Progress

- **Terminal wire keys (2026-08-05):** Assistant and tool-result metadata now
  serializes with pi-compatible camelCase keys (`stopReason`,
  `errorMessage`, `rawStopReason`, `toolCallId`, `toolName`, and
  `addedToolNames`), with round-trip coverage.
- **Role-tagged union wire (2026-08-05):** `AgentMessage` serialization now
  injects pi's required `user`, `assistant`, and `toolResult` role tags at the
  union boundary; all three variants round-trip through serde coverage.
- **Content wire parity (2026-08-05):** Image blocks now carry base64 strings
  with pi's `mimeType` key rather than byte arrays/`mime_type`, and thinking
  blocks serialize their payload as `thinking` rather than `text`. Focused
  serde coverage pins the exact JSON shape and round-trip behavior.
- **Assistant response metadata (2026-08-05):** Added pi's optional
  `responseModel`, `responseId`, and typed `diagnostics` fields, including
  diagnostic error/details payloads and complete serde round-trip coverage.
- **User content compatibility (2026-08-05):** User-message deserialization
  now accepts pi's string-content shorthand and normalizes it to one text
  block for the actor/event pipeline, with focused serde coverage.
- **Tool-call metadata (2026-08-05):** Added optional provider-specific
  `thoughtSignature` to tool calls and preserved it through argument
  preparation and event reconstruction, with wire-shape coverage.
- **Optional tool additions (2026-08-05):** Empty `addedToolNames` arrays are
  omitted from tool-result wire payloads like pi, while non-empty additions
  retain the camelCase key; serde coverage pins both forms.
- **Optional termination hint (2026-08-05):** `AgentToolResult.terminate`
  now omits `false` on the wire and retains `true`, matching pi's optional
  termination hint and error-result shape.
- **Error-result details parity (2026-08-05):** Synthetic, truncated, and
  executor-drop tool errors now use `{}` details, matching pi's
  `createErrorToolResult` payload rather than serializing JSON `null`.

## Acceptance

- `cargo test -p runie-core` green; new round-trip serde tests fill every new field and assert exact JSON keys match pi's TS shapes.
- `wire_to_agent` / `default_convert_to_llm` (convert.rs) preserve the new fields.
- Provider `replay.rs` populates `usage`/`raw_stop_reason` from SSE `done` payloads.
