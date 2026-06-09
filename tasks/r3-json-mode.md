# JSON Mode

**Status**: todo
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

- [ ] `runie-json` reads JSON from stdin
- [ ] Writes JSON response to stdout
- [ ] Supports streaming via JSONL (one object per chunk)
- [ ] Tool calls returned as structured JSON
- [ ] Exit code 0 on success, 1 on error
- [ ] Schema documented

## Tests

### Layer 1
- [ ] `json_mode_parses_request` — valid JSON parsed
- [ ] `json_mode_outputs_valid_json` — stdout is valid JSON
- [ ] `json_mode_returns_tool_calls` — tool calls in output
