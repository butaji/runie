# Fix tool execution timeout

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

Tool execution had no timeout. Only the `bash` tool had its own 60s timeout, but `list_dir`, `read_file`, `grep`, and all other tools could hang forever if the filesystem was slow or unresponsive.

## Root Cause

`execute_tool_call` in `tool_runner.rs` called `run_tool()` directly without any timeout wrapper. A slow NFS mount or network filesystem would block indefinitely.

## Fix

Added timeout wrapper around tool execution in `execute_tool_call`:

```rust
const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 30;

PermissionAction::Allow => {
    let duration = Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS);
    match timeout(duration, run_tool(tool, tool_call, ctx)).await {
        Ok(output) => output,
        Err(_) => ToolOutput {
            // Return timeout error after 30 seconds
            content: format!("Tool execution timed out after {} seconds", DEFAULT_TOOL_TIMEOUT_SECS),
            status: ToolStatus::Error,
            // ...
        },
    }
}
```

## Acceptance Criteria

- [x] All tools have 30s execution timeout (except bash which has its own configurable timeout)
- [x] Timeout returns `ToolStatus::Error` with clear error message
- [x] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [x] `tool_timeout_returns_error` — verifies timeout mechanism works

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- N/A

### Layer 4 — Smoke / E2E
- [x] "list all files in dir" no longer hangs forever

## Files touched

- `crates/runie-agent/src/tool_runner.rs`

## Notes

- Bash tool keeps its own 60s timeout via `timeout_seconds` parameter
- Other tools now get 30s default timeout
- Timeout is conservative to allow legitimate long operations while preventing indefinite hangs
