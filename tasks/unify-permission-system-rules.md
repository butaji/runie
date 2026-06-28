# Unify permission-system rule engines

**Status**: todo
**Milestone**: R2
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/permissions/` contains two parallel engines: `PermissionSet` (last-match-wins rules with scope precedence) and `PermissionManager` policy chain (`DefaultToolApprove`, `FileAccessAsk`). They evaluate the same concepts with different abstractions, and `ApprovalRegistry` is a custom `Mutex<HashMap<String, oneshot::Sender>>`. Unifying them into one declarative ruleset will remove conceptual duplication and ~250 lines.

## Acceptance Criteria

- [ ] Merge `PermissionSet` rule evaluation and the `PermissionManager` policy chain into a single ruleset abstraction.
- [ ] Keep the actor for async approval flows, but simplify `ApprovalRegistry` using `tokio::sync::watch` or `mpsc`.
- [ ] Preserve all existing permission decisions (default allow/deny, file-access ask, tool-specific rules).
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `default_allow_for_safe_tools` — safe tools are approved by default.
- [ ] `file_access_triggers_ask` — file-access outside the sandbox triggers an approval request.
- [ ] `explicit_deny_overrides` — a deny rule wins over a default allow.

### Layer 2 — Event Handling
- [ ] `permission_actor_returns_decision` — approval request events produce the expected decision event.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-core/src/permissions/rules.rs`
- `crates/runie-core/src/permissions/gate.rs`
- `crates/runie-core/src/permissions/default_tool_approve.rs`
- `crates/runie-core/src/permissions/file_access_ask.rs`
- `crates/runie-core/src/permissions/approval_registry.rs`

## Notes

- `PermissionManager::new(_mode)` currently ignores its mode argument; use `PermissionMode` to assemble the default policy chain.
- `PermissionResult` is redundant with `PermissionAction`; keep one enum.
- This is a conceptual unification, not a security policy change. Keep the same external behavior.
- Coordinate with `collapse-actor-handles-to-typed-map.md` because `PermissionActor` handle wiring may change.
