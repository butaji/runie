# Permission System Runtime Wiring

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: none
**Blocks**: `headless-approval-defaults`

## Description

`PermissionManager`, `ApprovalSink`, `TuiApprovalSink`, and wildcard `PermissionSet` are fully implemented and unit-tested, but the agent turn loop and headless/server runners call `tool.call(...)` directly without consulting them. `Tool::requires_approval()` is defined but never invoked, so the auto/ask/deny policy chain and sensitive-path denylist have no effect at runtime.

## Acceptance Criteria

- [x] `PermissionManager::evaluate` is called before every tool invocation in `runie-agent/src/turn.rs`.
- [x] Headless (`runie-json`, `runie-server`) and server modes consult the permission manager.
- [x] `Ask` decisions are routed through an `ApprovalSink`; TUI mode surfaces a dialog and awaits user input before executing.
- [x] Denied tools return an error result without side effects.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo clippy --workspace -- -D warnings` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `permission_manager_denies_destructive_tool` — deny result stops execution.
- [x] `permission_manager_auto_allows_read_only_tool` — read-only tools execute without prompt.

### Layer 2 — Event Handling
- [x] `tool_call_emits_permission_request_event` — pending approval emits a bus event.
- [x] `approval_event_resumes_tool_execution` — approval answer event resumes the turn.

### Layer 3 — Rendering
- [x] `approval_dialog_renders_tool_and_args` — TUI dialog shows the tool call awaiting approval.

### Layer 4 — Smoke / Crash
- [x] `smoke_denied_write_file_does_not_create_file` — denied `write_file` leaves disk untouched.

## Files touched

- `crates/runie-agent/src/turn.rs`
- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-engine/src/tool/mod.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`

## Notes

This is the most severe safety gap from the review. Non-interactive modes should default to denying destructive tools unless an explicit flag is provided (see `headless-approval-defaults`).
