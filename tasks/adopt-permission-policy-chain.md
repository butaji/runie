# Adopt Permission Policy Chain

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Refactor permission system to use chain-of-responsibility pattern with policy matching:

```rust
pub trait PermissionPolicy: Send + Sync {
    fn name(&self) -> &str;
    fn matches(&self, ctx: &PermissionContext) -> bool;
    async fn evaluate(&self, ctx: &PermissionContext) -> Option<PermissionResult>;
}

pub struct PermissionManager {
    policies: Vec<Box<dyn PermissionPolicy>>,
    mode: PermissionMode,  // yolo | manual | auto
}
```

Built-in policies:
- `DefaultToolApprove` — Safe tools auto-approved
- `GitTrackedWriteApprove` — Auto-approve writes to git-tracked files
- `FileAccessAsk` — Prompt for file access outside cwd
- `SessionApprovalHistory` — Cache approvals within session
- `HookPolicy` — Hook-based custom policies

Reference: `~/Code/agents/kimi-code/packages/agent-core/src/permission/`

## Acceptance Criteria

- [x] `PermissionPolicy` trait with `name()`, `matches()`, `evaluate()`.
- [x] `PermissionManager` evaluates policies in order (first-match-wins).
- [x] Built-in policies implemented: DefaultToolApprove, GitTrackedWrite, FileAccessAsk.
- [x] Policy configuration via config file (`[permissions]` section with `mode`).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `permission_policy_chain_first_match_wins` — first matching policy used.
- [x] `default_tool_approve_allows_safe_tools` — safe tools auto-approved.
- [x] `git_tracked_write_approve_passes_git_files` — git-tracked files approved.
- [x] `file_access_ask_requires_approval` — non-cwd files prompt.

### Layer 2 — Event Handling
- [ ] `permission_request_emits_event` — pending request event emitted.
- [ ] `permission_approval_resumes_tool` — approval resumes execution.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/permissions/` (refactor existing)
- `crates/runie-core/src/config.rs` (`PermissionsConfig` section)
- `config.schema.json`

## Notes

Chain-of-responsibility enables extensibility without modifying core permission logic. Legacy `PermissionSet`/`PermissionRule` API preserved and re-exported from `permissions/mod.rs`.

## Test Results

- `cargo test --workspace`: passed (workspace tests + doc tests)
- `cargo clippy --workspace -- -D warnings`: passed
- Linter (build.rs): no new file/function/complexity violations
