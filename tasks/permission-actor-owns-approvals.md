# PermissionActor owns approval registry and request UI

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

Permission approvals use a shared `Arc<Mutex<ApprovalRegistry>>` and a `permission_request` UI field that various handlers mutate directly. There is no `PermissionActor`. Create one that owns both the registry and the UI request state.

Current violators:
- `model/state/app_state.rs` — initializes `approval_registry`, preserves it across resets.
- `update/permission.rs` — sets `state.permission_request`.
- `update/input/mod.rs` — resolves the registry and clears `permission_request` on modal input.
- `commands/dsl/handlers/session/mod.rs` — `/new` clears `permission_request` directly.
- `permissions/approval_registry.rs` — the registry itself is a shared mutable struct.
- `runie-agent/src/emit_approval_sink.rs` — registers oneshots directly in the registry.

## Acceptance criteria

- [ ] `PermissionActor` is an mpsc actor owning the `ApprovalRegistry` and the current `permission_request` UI state.
- [ ] `PermissionMsg` covers: `AskPermission { request_id, tool, input }`, `ResolvePermission { request_id, action }`, `CancelPermission`, `DismissRequest`.
- [ ] `AppState.permission_request` is private; reads go through an immutable accessor.
- [ ] `AppState.approval_registry` is removed; only `PermissionActor` holds the registry.
- [ ] `PermissionActor` emits facts: `PermissionRequest { request }`, `PermissionResolved { request_id, action }`, `PermissionRequestDismissed`.
- [ ] `runie-agent/src/emit_approval_sink.rs` sends `PermissionMsg::AskPermission` instead of registering directly.
- [ ] UI input handler sends `PermissionMsg::ResolvePermission` instead of calling `registry.lock().resolve(...)`.
- [ ] `/new` sends `PermissionMsg::CancelPermission` instead of clearing the field.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `permission_actor_ask_creates_request` — `AskPermission` stores pending oneshot and emits `PermissionRequest`.
- [ ] `permission_actor_resolve_sends_action` — `ResolvePermission` resolves the oneshot and dismisses UI state.

### Layer 2 — Event Handling
- [ ] `agent_tool_call_asks_permission` — agent emits `PermissionMsg::AskPermission`.
- [ ] `modal_enter_resolves_permission` — Enter in permission modal sends `PermissionMsg::ResolvePermission`.

### Layer 3 — Rendering
- [ ] `permission_request_renders_modal` — `PermissionRequest` fact renders the approval dialog.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_tool_approval_flow_routes_through_permission_actor` — a tool that requires approval blocks until resolved, with no direct registry access.

## Files touched

- `crates/runie-core/src/actors/permission/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/permissions/approval_registry.rs` — move into actor or keep as internal helper.
- `crates/runie-core/src/model/state/app_state.rs` — remove `approval_registry`, private `permission_request`.
- `crates/runie-core/src/update/permission.rs` — emit `PermissionMsg`.
- `crates/runie-core/src/update/input/mod.rs` — modal resolution emits `PermissionMsg`.
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — `/new` emits `PermissionMsg::CancelPermission`.
- `crates/runie-agent/src/emit_approval_sink.rs` — send `PermissionMsg::AskPermission`.

## Notes

- The registry must still be accessible from `runie-agent`. Use an mpsc handle or a thin async client that sends `PermissionMsg`.
- Keep the actual permission policy logic (`PermissionGate`, `AutoAllowSink`) in its current crate; only the registry/request coordination moves to the actor.
