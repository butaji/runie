# JSON Mode

**Status**: done
**Milestone**: R3
**Category**: Modes

## Description

Structured JSON output for scripting and piping.

## Architecture

```rust
// Separate binary: runie-json
// Usage: echo '{"prompt": "hello"}' | runie-json

#[derive(Serialize, Deserialize)]
pub struct JsonRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub tools: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct JsonResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tokens_used: usize,
    pub duration_ms: u64,
}
```

## Acceptance Criteria

- [x] `runie-json` reads JSON from stdin
- [x] Writes JSON response to stdout
- [x] Supports streaming via JSONL (one object per chunk)
- [x] Tool calls returned as structured JSON
- [x] Exit code 0 on success, 1 on error
- [x] Schema documented

## Tests

### Layer 1
- [x] `json_mode_parses_request` — valid JSON parsed
- [x] `json_mode_outputs_valid_json` — stdout is valid JSON
- [x] `json_mode_returns_tool_calls` — tool calls in output
