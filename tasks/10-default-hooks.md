# Step 10: Default hooks

**Status:** implemented (2026-08-07)
**Depends on:** 09

## Goal
Provide the four default hooks the loop uses when the user does not supply them.

## Changes
- `crates/runie-core/src/hooks.rs` (new file):
  - `pub fn default_convert_to_llm(messages: &[AgentMessage]) -> Vec<WireMessage>`: filters Custom (drops them), maps User/Assistant/ToolResult to their wire shape.
  - `pub async fn default_transform_context(messages: Vec<AgentMessage>) -> Vec<AgentMessage>`: identity.
  - `pub async fn default_before_tool_call(_ctx: BeforeToolCallContext) -> Option<BeforeToolCallResult>`: returns None (allow).
  - `pub async fn default_after_tool_call(_ctx: AfterToolCallContext) -> Option<AfterToolCallResult>`: returns None (no override).
- Hook trait objects: `Arc<dyn Fn(...) -> BoxFuture<...> + Send + Sync>`.
- Re-export from `lib.rs`.

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test: `default_convert_to_llm` with mixed messages returns 2 wire messages (drops Custom).

## Notes
- `WireMessage` lives in `convert.rs` alongside the default convert fn.
