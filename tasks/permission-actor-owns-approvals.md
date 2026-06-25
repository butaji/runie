# PermissionActor owns approval registry and request UI

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

Permission approvals use a `PermissionActor` that owns both the registry and the UI request state. The actor is already an mpsc actor with proper message handling.

## What was done

The `PermissionActor` was created with:
- `ApprovalRegistry` ownership
- `PermissionRequestState` ownership (stored as `perm_req` field)
- `PermissionMsg` variants: `AskPermission`, `ResolvePermission`, `CancelPermission`, `DismissRequest`
- Proper fact emission: `PermissionRequest`, `PermissionResponse`, `PermissionRequestDismissed`

The `runie-agent/src/emit_approval_sink.rs` sends `PermissionMsg::AskPermission` to the actor.

UI input handler (`update/input/mod.rs`) uses `handles.try_resolve_permission()` to emit intents.

Session `/new` command (`commands/dsl/handlers/session/mod.rs`) uses `handles.try_dismiss_permission()` to dismiss pending requests.

The `AppState.perm_req` field is accessible via `permission_request_opt()` (immutable) and `permission_request_mut()` (mutable) accessors. The field must remain `pub` for struct literals in tests across crates.

## Acceptance criteria

- [x] `PermissionActor` is an mpsc actor owning the `ApprovalRegistry` and the current `permission_request` UI state.
- [x] `PermissionMsg` covers: `AskPermission { request_id, tool, input }`, `ResolvePermission { request_id, action }`, `CancelPermission`, `DismissRequest`.
- [x] `AppState.permission_request` is accessible via accessor methods; reads go through `permission_request_opt()`.
- [x] `AppState.approval_registry` is not in AppState; only `PermissionActor` holds the registry.
- [x] `PermissionActor` emits facts: `PermissionRequest`, `PermissionResponse`, `PermissionRequestDismissed`.
- [x] `runie-agent/src/emit_approval_sink.rs` sends `PermissionMsg::AskPermission` instead of registering directly.
- [x] UI input handler sends `PermissionMsg::ResolvePermission` via actor handles.
- [x] `/new` sends `PermissionMsg::CancelPermission` via actor handles.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `permission_actor_ask_creates_request` — `AskPermission` stores pending oneshot and emits `PermissionRequest`.
- [x] `permission_actor_resolve_sends_action` — `ResolvePermission` resolves the oneshot and dismisses UI state.

### Layer 2 — Event Handling
- [x] `agent_tool_call_asks_permission` — agent emits `PermissionMsg::AskPermission` via `EmitApprovalSink`.
- [x] `modal_enter_resolves_permission` — y/n/a keys send intents via `handles.try_resolve_permission()`.

### Layer 3 — Rendering
- [x] `permission_request_renders_modal` — `PermissionRequest` fact renders the approval dialog (popups/permission.rs).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_tool_approval_flow_routes_through_permission_actor` — integration tests verify the flow routes through the actor.

## Files touched

- `crates/runie-core/src/actors/permission/` — actor implementation
- `crates/runie-core/src/model/state/app_state.rs` — renamed field to `perm_req`, added accessors
- `crates/runie-core/src/model/state/accessors.rs` — added `permission_request_opt()` and `permission_request_mut()`
- `crates/runie-core/src/update/permission.rs` — updated to use accessor
- `crates/runie-core/src/update/input/mod.rs` — uses `handles.try_resolve_permission()`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — uses `handles.try_dismiss_permission()`
- `crates/runie-agent/src/emit_approval_sink.rs` — sends `PermissionMsg::AskPermission`
- `crates/runie-core/src/update/input/tests.rs` — updated to use accessor
- `crates/runie-core/src/tests/slash/session.rs` — updated to use accessor

## Notes

- The field is `pub` (required for struct literals in tests across crates); access is routed through accessors.
- The registry must still be accessible from `runie-agent`. Uses an mpsc handle that sends `PermissionMsg`.
- Keep the actual permission policy logic (`PermissionGate`, `AutoAllowSink`) in its current crate; only the registry/request coordination is in the actor.
