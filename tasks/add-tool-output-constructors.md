# Add ToolOutput constructor helpers

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Every tool implementation manually constructs `ToolOutput { tool_name, tool_args, content, bytes_transferred, duration, status }` in success paths, error paths, and lock-error paths. Fields like `bytes_transferred: None` and `duration: start.elapsed()` are repeated everywhere. Adding a new field becomes a workspace-wide refactor.

## Acceptance Criteria

- [ ] `runie_core::tool::ToolOutput` gains constructor helpers: `success`, `error`, `json_success`, `json_error`.
- [ ] All tool implementations use the helpers instead of struct literals.
- [ ] Existing behavior is preserved.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_output_success_sets_duration_and_status` — constructor fills fields correctly.
- [ ] `tool_output_json_error_serializes_message` — `json_error` produces expected JSON.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `bash_tool_error_uses_helper` — a failing bash tool returns the same output shape as before.

## Files touched

- `crates/runie-core/src/tool/context.rs` or `crates/runie-core/src/tool/mod.rs`
- `crates/runie-engine/src/tool/*.rs`

## Notes

Many tools already use `tool_error()` for simple cases; extend that pattern rather than inventing a new naming scheme.
