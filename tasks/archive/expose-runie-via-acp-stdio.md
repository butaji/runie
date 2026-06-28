# Expose Runie via ACP over stdio

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: app-state-read-only-projection
**Blocks**: standardize-headless-output-streaming-json

## Summary

Added an ACP (Agent Client Protocol) stdio adapter that exposes Runie's event bus as JSON-RPC 2.0 over stdin/stdout. The TUI becomes one client; IDEs and scripts can reuse the same agent runtime without custom integration code.

## Implementation

- Added `crates/runie-cli/src/acp.rs` module implementing the ACP adapter
- Added `runie acp` subcommand to `runie-cli`
- ACP reads JSON-RPC 2.0 requests from stdin and writes events as JSON-RPC notifications to stdout

## JSON-RPC Methods

- `initialize` → returns version info
- `submit_input` → sends user input and waits for turn completion
- `interrupt` → aborts the current turn
- `permission_resp` → responds to permission requests
- `shutdown` → gracefully shuts down

## Events (JSON-RPC Notifications)

- `turn_complete`, `text_start`, `text_end`, `text_delta`
- `thinking_start`, `thinking_end`, `thinking_delta`
- `tool_start`, `tool_end`
- `permission_request`
- `error`, `end`, `shutdown`

## Acceptance Criteria

- [x] `runie acp` subcommand starts the ACP adapter
- [x] ACP messages are JSON-RPC 2.0 over stdin/stdout
- [x] Intents can be sent into the system; facts/events are streamed out
- [x] Existing TUI and headless modes continue to work unchanged
- [x] Authentication and permission gates are preserved
- [x] `cargo check --workspace` is green

## Tests

- **Layer 1**: ACP initialize returns correct version info
- **Layer 1**: ACP parameter parsing for submit_input, permission_resp
- **Layer 1**: Event to notification conversion for TurnComplete, ToolStart, Input events
- **Layer 4**: ACP message round-trip tests

## Files touched

- `crates/runie-cli/src/acp.rs` — new ACP adapter module
- `crates/runie-cli/src/main.rs` — added `acp` subcommand

## Notes

- The ACP adapter spawns the full actor system (ConfigActor, ProviderActor, SessionActor, IoActor, PermissionActor, AgentActor) just like the TUI
- Events from the bus are forwarded both to stdout (as JSON-RPC notifications) and to a local channel (for request handlers)
- The implementation is intentionally minimal to keep the protocol clean
