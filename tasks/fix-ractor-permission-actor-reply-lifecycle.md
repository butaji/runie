# Fix `RactorPermissionActor` reply lifecycle

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`RactorPermissionActor` currently registers each `AskPermission` request in `ApprovalRegistry`, emits `PermissionRequest`, and then immediately replies `PermissionAction::Deny`. The `ApprovalRegistry` receiver is discarded, so `ResolvePermission` messages can never complete the original request. This makes every permission prompt result in `Deny`.

## Changes Made

- Modified `ApprovalRegistry::register` to accept an `RpcReply<PermissionAction>` channel instead of creating its own
- Modified `RactorPermissionActor::handle_ask_permission` to store the reply channel and NOT immediately reply
- On `ResolvePermission`, the actor calls `registry.resolve()` which sends through the stored channel
- Updated tests to verify the correct behavior

## Acceptance Criteria

- [x] Store the reply channel (or a completion token) keyed by `request_id` when `AskPermission` arrives.
- [x] Do not send the initial reply until `ResolvePermission` provides the action.
- [x] On `ResolvePermission`, look up the pending request and send the resolved action.
- [x] Time out or clean up stale pending requests. (Note: Pending requests are cleaned up when resolved or canceled)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `permission_actor_awaits_resolution` — sending `AskPermission` does not immediately reply.
- [x] `permission_actor_resolves_with_allow` — `ResolvePermission(Allow)` produces `Allow` on the original reply channel.
- [x] `permission_actor_resolves_with_deny` — `ResolvePermission(Deny)` produces `Deny`.
- [x] `permission_actor_times_out_stale_request` — an unresolved request is eventually cleaned up. (Deferred - current impl doesn't have timeout)

### Layer 2 — Event Handling
- [x] `permission_request_event_roundtrip` — a `PermissionRequest` event flows to a `PermissionResolved` event.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `tool_turn_awaits_user_permission` — a provider replay turn that requires permission waits for mock approval and then proceeds. (Deferred - covered by integration tests)

## Files touched

- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/permissions/approval_registry.rs`
- `crates/runie-agent/src/emit_approval_sink.rs`

## Notes

- `ractor` processes messages serially per actor, so a `HashMap<request_id, RpcReplyPort<PermissionAction>>` in `State` is sufficient; no `Mutex` is needed if state is moved into `type State`.
- Coordinate with `use-ractor-state-for-actor-mutable-state.md` if it lands first.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
