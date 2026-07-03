# Round 1 — SSOT Actors & State Ownership

## Findings

### 1. `AgentState` is mutated directly even though `TurnState` is authoritative

`AgentState` is documented as a read-only projection of `TurnState`, but update handlers mutate it directly:

- `crates/runie-core/src/update/agent/core_messages.rs:70,96,176-184,271-286` — sets `current_request_id`, `last_assistant_index`, `turn_active`, `streaming`, `inflight`, etc.
- `crates/runie-core/src/update/system.rs:181-213` — `apply_turn_aborted` writes `turn_active`, `streaming`, `inflight`, `current_request_id`, `current_tool_name`, `current_action`, `turn_started_at`, `thinking_started_at`, `tool_started_at` into both `turn_state` and `agent`.
- `crates/runie-core/src/update/session.rs:199-271` — `apply_queue_delivery_sync` duplicates `RactorTurnActor::handle_deliver_queued` logic and pushes directly into `AgentState` queues.

### 2. `TurnState`/`AgentState` duplication

`AppState` carries both `turn_state` (authoritative) and `agent` (projection). The projection is kept in sync by hand in `apply_turn_aborted` and similar helpers. This is mirrored state, which violates the SSOT ADR.

### 3. Permission dismissal is mutated locally in the TUI

`crates/runie-tui/src/ui_actor.rs:377` — `UiActor` clears `permission_request_mut()` directly after sending the resolve message, instead of waiting for `PermissionActor` to emit `PermissionRequestDismissed`.

### 4. Projection bypasses for environment facts

`crates/runie-core/src/update/dispatch.rs:306-309` assigns `git_info`/`cwd_name` directly on `Event::EnvDetected`. `domain_ops.rs:14-22` also exposes `set_git_info`/`set_cwd_name` setters that write the projection directly.

## Recommended changes

1. Make `TurnActor` the sole owner of turn/queue/inflight state. `AgentState` becomes a typed projection derived from `TurnState` events, with no public setters.
2. Delete the synchronous `TurnQueue` fallback in `AppState::apply_queue_delivery_sync`; always route queued delivery through `TurnActor`.
3. Remove direct `permission_request_mut()` writes from `UiActor`; react only to `PermissionRequestDismissed` events.
4. Route `EnvDetected` through the accessor layer and, if needed, an owning actor (e.g., `IoActor`/`EnvActor`) rather than direct field assignment.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Merge `AgentState` into `TurnState` projection | `tasks/merge-agentstate-into-turnstate-projection.md` | existing `todo` |
| Remove direct `AppState` mutation from TUI/command handlers | `tasks/remove-direct-appstate-mutation-from-tui-handlers.md` | existing `todo` |
| Remove direct `AppState` mutation from core update handlers | `tasks/remove-direct-appstate-mutation-from-core-update-handlers.md` | **new** |
| Move approval registry into `PermissionActor` state | `tasks/move-approval-registry-into-permission-actor-state.md` | existing `todo` |
| Remove environment projection bypasses | `tasks/remove-direct-projection-bypasses-in-dispatch-and-domain-ops.md` | **new** |
