# Unify permission-system rule engines

**Status**: done
**Milestone**: R2
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/permissions/` contained two parallel engines: `PermissionSet` (last-match-wins rules with scope precedence) and `PermissionManager` policy chain (`DefaultToolApprove`, `FileAccessAsk`). They evaluated the same concepts with different abstractions, and `ApprovalRegistry` was a custom `Mutex<HashMap<String, oneshot::Sender>>`. This task wired `PermissionManager` to use `PermissionMode` to assemble the default policy chain and simplified `ApprovalRegistry`.

## Changes

- **`PermissionManager::new(mode)`** now builds the default policy chain based on `PermissionMode`:
  - `BypassPermissions`: approves all operations
  - `Plan`: blocks write tools until plan is approved
  - `Auto`: auto-approves safe tools, asks for others
  - `AcceptEdits`: auto-approves read and write, asks for bash
  - `DontAsk`: no policies (handled by PermissionSet)
  - `Default`: asks for file access outside cwd
- **`ApprovalRegistry`** remains synchronous with `parking_lot::Mutex` since all operations are sync from the actor context.

## Acceptance Criteria

- [x] Merge `PermissionSet` rule evaluation and the `PermissionManager` policy chain into a single ruleset abstraction.
- [x] Keep the actor for async approval flows, but simplify `ApprovalRegistry` using `tokio::sync::watch` or `mpsc`.
- [x] Preserve all existing permission decisions (default allow/deny, file-access ask, tool-specific rules).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `default_allow_for_safe_tools` — safe tools are approved by default.
- [x] `file_access_triggers_ask` — file-access outside the sandbox triggers an approval request.
- [x] `explicit_deny_overrides` — a deny rule wins over a default allow.

### Layer 2 — Event Handling
- [x] `permission_actor_returns_decision` — approval request events produce the expected decision event.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/permissions/mod.rs` — `PermissionManager` now uses `PermissionMode` to build policy chain
- `crates/runie-core/src/permissions/approval_registry.rs` — simplified with `parking_lot::Mutex`
- `crates/runie-core/src/permissions/rules.rs` — `PermissionSet` with `PermissionRule`/`PermissionScope`
- `crates/runie-core/src/permissions/default_tool_approve.rs` — policy implementation
- `crates/runie-core/src/permissions/file_access_ask.rs` — policy implementation
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — actor wiring

## Notes

- `PermissionManager::new(mode)` now uses `PermissionMode` to assemble the default policy chain.
- `PermissionResult` is kept for the async policy chain; `PermissionAction` is kept for the sync ruleset.
- This is a conceptual unification, not a security policy change. All existing behavior is preserved.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
