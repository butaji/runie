# Fix provider client timeout

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

The OpenAI provider created `reqwest::Client::new()` without any timeout configuration. If the API was unreachable or hung, requests would never complete, causing tool execution to hang forever.

## Root Cause

`OpenAiProvider::new()` and the stream module used `reqwest::Client::new()` which has no default timeout. Network issues or API unavailability would cause indefinite hangs.

## Fix

1. Added `client: reqwest::Client` field to `OpenAiProvider` struct
2. Configured timeouts in `new()`:
   - 120s request timeout
   - 10s connect timeout
   - Falls back to unconfigured client if builder fails
3. Updated stream module to use provider's client instead of creating new one

```rust
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
    model_meta: Option<&'static runie_core::provider_registry::ModelMeta>,
    tools: Vec<serde_json::Value>,
    tool_choice: Option<serde_json::Value>,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        // ...
    }
}
```

## Acceptance Criteria

- [x] Provider client has request timeout (120s)
- [x] Provider client has connect timeout (10s)
- [x] Stream module uses provider's client
- [x] `cargo test --workspace` succeeds
- [x] Tool calls no longer hang indefinitely

## Tests

### Layer 1 — State/Logic
- N/A (configuration changes, logic unchanged)

### Layer 2 — Event Handling
- N/A (timeout is at HTTP layer)

### Layer 3 — Rendering
- N/A (no UI changes)

### Layer 4 — Smoke / E2E
- [x] Manual verification: "list all files in dir" no longer hangs
- [x] All workspace tests pass

## Files touched

- `crates/runie-provider/src/openai/mod.rs`
- `crates/runie-provider/src/openai/stream.rs`

## Notes

This fix prevents indefinite hangs when the API is unreachable. The timeouts are conservative (120s) to allow for legitimate long-running requests while protecting against network failures.
