# Step 03: Port core types

**Status:** pending
**Depends on:** 02

## Goal
Port the type definitions from `pi-agent-core/src/types.ts` to Rust with `serde` derives where appropriate.

## Changes
- `crates/runie-core/src/types.rs`:
  - `ThinkingLevel` enum (`Off`, `Minimal`, `Low`, `Medium`, `High`, `XHigh`, `Max`).
  - `ToolExecutionMode` enum (`Sequential`, `Parallel`).
  - `QueueMode` enum (`OneAtATime`, `All`).
  - `StopReason` enum (`Stop`, `ToolUse`, `MaxTokens`, `Error`, `Aborted`).
  - `TextContent` struct `{ text: String }`.
  - `ImageContent` struct `{ data: Vec<u8>, mime_type: String }`.
  - `Usage` struct (input/output/cache_read/cache_write/total_tokens/cost).
  - `UserMessage`, `AssistantMessage`, `ToolResultMessage`, `ToolCall`, `ToolResultContent` structs.
  - `AgentMessage` enum: `User(UserMessage)`, `Assistant(AssistantMessage)`, `ToolResult(ToolResultMessage)`, `Custom(Box<dyn AgentMessageExt>)`.
  - `AgentMessageExt` trait with `fn role()`, `fn timestamp()`, `fn convert_to_llm() -> Option<...>`.
  - `AgentContext` struct `{ system_prompt, messages, tools }`.
  - `AgentState` struct (mutable fields: `system_prompt`, `model`, `thinking_level`, `messages`, `tools`; computed: `is_streaming`, `streaming_message`, `pending_tool_calls`, `error_message`).
  - `AgentTool<P, D>` trait: `name`, `label`, `description`, `parameters`, `prepare_arguments`, `execute`, `execution_mode`.
  - `AgentToolResult<D>` struct: `content`, `details`, `usage`, `added_tool_names`, `terminate`.
  - `AgentEvent` enum: `AgentStart`, `AgentEnd { messages }`, `TurnStart`, `TurnEnd { message, tool_results }`, `MessageStart { message }`, `MessageUpdate { message, event }`, `MessageEnd { message }`, `ToolExecutionStart { tool_call_id, tool_name, args }`, `ToolExecutionUpdate { tool_call_id, tool_name, args, partial_result }`, `ToolExecutionEnd { tool_call_id, tool_name, result, is_error }`.
  - `BeforeToolCallResult`, `AfterToolCallResult`, hook context types.
- `crates/runie-core/src/convert.rs`: `default_convert_to_llm` (filters Custom, passes through User/Assistant/ToolResult).

## Verification
- `cargo check -p runie-core` → exit 0.
- `cargo doc -p runie-core --no-deps` → exit 0.

## Notes
- `serde` derives on wire types; actor-internal types don't need them.
- `AgentMessageExt` uses `Box<dyn ...>` for `Custom`; document lifetime constraint `'static`.
- Pin pi-agent-core commit hash in a top-of-file comment.