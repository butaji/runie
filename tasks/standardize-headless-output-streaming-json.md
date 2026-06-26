# Standardize headless output as streaming JSON events

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: expose-runie-via-acp-stdio
**Blocks**: none

## Summary

Replace custom headless CLI output with a single streaming JSON event stream. All clients — TUI, headless scripts, ACP consumers — read the same fact/event shape.

## Event schema

Inspired by Grok Build's headless output, Runie events are newline-delimited JSON:

```json
{"type":"text","data":"Hello, "}
{"type":"text","data":"world!"}
{"type":"tool_call_start","id":"call_1","name":"bash"}
{"type":"tool_call_input_delta","id":"call_1","delta":"{\"cmd\":\"ls\"}"}
{"type":"tool_call_end","id":"call_1"}
{"type":"permission_request","id":"perm_1","tool":"bash","args":{...}}
{"type":"tool_result","id":"call_1","output":"..."}
{"type":"usage","input_tokens":120,"output_tokens":45}
{"type":"end","stopReason":"EndTurn","sessionId":"...","requestId":"..."}
```

Error events:

```json
{"type":"error","message":"Agent building failed: ..."}
```

## Acceptance Criteria

- Headless mode (`runie -p "..."`) emits newline-delimited JSON events.
- Event types cover: `text`, `thinking`, `tool_call_start`, `tool_call_input_delta`, `tool_call_end`, `permission_request`, `tool_result`, `usage`, `error`, `end`.
- Custom progress/formatting modules in `runie-cli` are removed.
- TUI can consume the same stream internally.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Snapshot tests for streaming JSON event serialization.
- **Layer 4**: Headless run with captured provider fixture produces expected JSON lines.
