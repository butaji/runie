# Fix `RactorPermissionActor` reply lifecycle

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`RactorPermissionActor` currently registers each `AskPermission` request in `ApprovalRegistry`, emits `PermissionRequest`, and then immediately replies `PermissionAction::Deny`. The `ApprovalRegistry` receiver is discarded, so `ResolvePermission` messages can never complete the original request. This makes every permission prompt result in `Deny`.

## Acceptance Criteria

- [ ] Store the reply channel (or a completion token) keyed by `request_id` when `AskPermission` arrives.
- [ ] Do not send the initial reply until `ResolvePermission` provides the action.
- [ ] On `ResolvePermission`, look up the pending request and send the resolved action.
- [ ] Time out or clean up stale pending requests.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `permission_actor_awaits_resolution` — sending `AskPermission` does not immediately reply.
- [ ] `permission_actor_resolves_with_allow` — `ResolvePermission(Allow)` produces `Allow` on the original reply channel.
- [ ] `permission_actor_resolves_with_deny` — `ResolvePermission(Deny)` produces `Deny`.
- [ ] `permission_actor_times_out_stale_request` — an unresolved request is eventually cleaned up.

### Layer 2 — Event Handling
- [ ] `permission_request_event_roundtrip` — a `PermissionRequest` event flows to a `PermissionResolved` event.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tool_turn_awaits_user_permission` — a provider replay turn that requires permission waits for mock approval and then proceeds.

## Files touched

- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/permissions/approval_registry.rs`
- `crates/runie-agent/src/emit_approval_sink.rs`

## Notes

- `ractor` processes messages serially per actor, so a `HashMap<request_id, RpcReplyPort<PermissionAction>>` in `State` is sufficient; no `Mutex` is needed if state is moved into `type State`.
- Coordinate with `use-ractor-state-for-actor-mutable-state.md` if it lands first.
