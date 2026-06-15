# Fix Bash Tool Timeout

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

`crates/runie-core/src/tool/bash.rs` parses `timeout_seconds` from the tool input but never uses it. `run_bash_inner` calls `Command::output()` without any timeout, so a long-running or hung shell command can block the blocking thread indefinitely.

## Acceptance Criteria

- [ ] `run_bash_inner` enforces the `timeout` passed from `call`.
- [ ] When a command exceeds the timeout it is killed and the result reports the timeout (e.g., a `TimedOut` status variant or a clear timeout message with `Error` status).
- [ ] Default timeout remains 60 seconds when `timeout_seconds` is omitted.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `bash_tool_respects_timeout_seconds` — `sleep 10` with `timeout_seconds: 1` returns within ~2 seconds and reports timeout.
- [ ] `bash_tool_runs_quick_command` — `echo hello` with default timeout succeeds and contains output.

### Layer 2 — Event Handling
N/A — tool execution only.

### Layer 3 — Rendering
N/A — no UI change.

### Layer 4 — Smoke / Crash
N/A — covered by Layer 1.

## Files touched

- `crates/runie-core/src/tool/bash.rs`
- `crates/runie-core/src/tool/mod.rs` (if a `TimedOut` variant is added to `ToolStatus`)

## Notes

- The runie-agent crate already has a `run_command_with_timeout` helper that can serve as a reference implementation.
- Keep the implementation inside the existing `spawn_blocking` call.
