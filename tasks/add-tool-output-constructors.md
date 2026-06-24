# Add ToolOutput constructor helpers

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Every tool implementation manually constructs `ToolOutput { tool_name, tool_args, content, bytes_transferred, duration, status }` in success paths, error paths, and lock-error paths. Fields like `bytes_transferred: None` and `duration: start.elapsed()` are repeated everywhere. Adding a new field becomes a workspace-wide refactor.

## Acceptance Criteria

- [x] `runie_core::tool::ToolOutput` gains constructor helpers: `success`, `error`, `json_success`, `json_error`, `blocked`, `success_with_bytes`.
- [x] Tool implementations (read_file, write_file, bash) use the helpers instead of struct literals.
- [x] Existing behavior is preserved.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `tool_output_success_sets_duration_and_status` — constructor fills fields correctly.
- [x] `tool_output_json_error_serializes_message` — `json_error` produces expected JSON.
- [x] `tool_output_blocked_sets_blocked_status` — blocked helper produces correct output.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `bash_tool_error_uses_helper` — a failing bash tool returns the same output shape as before.

## Files touched

- `crates/runie-core/src/tool/context.rs` — added constructors
- `crates/runie-engine/src/tool/read_file.rs` — uses helpers
- `crates/runie-engine/src/tool/write_file.rs` — uses helpers
- `crates/runie-engine/src/tool/bash.rs` — uses blocked helper

## Notes

Constructors added: `success`, `success_with_bytes`, `error`, `blocked`, `json_success`, `json_error`. Duration is set to `Duration::ZERO` by default; callers can override. Three tools updated to use helpers.
