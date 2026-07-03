# Round 3 — Actor Message Protocols

## Findings

### 1. Unsafe zeroed reply port

`crates/runie-core/src/update/session.rs:204-212` uses `unsafe { std::mem::zeroed() }` for the `RpcReplyPort` when sending `TurnMsg::DeliverQueued` fire-and-forget.

This is undefined behavior and a reliability hazard.

### 2. `Clone` zeroes reply port

`crates/runie-core/src/actors/turn/messages.rs:97-102` — `TurnMsg::DeliverQueued` derives `Clone`, which zeros the reply port. A cloned message cannot be used for RPC.

### 3. Race between actor commit and projection update

`crates/runie-core/src/update/dispatch.rs:137-144` updates `AppState` before `TurnActor` processes the message. Later events from `TurnActor` are applied on top of already-moved state.

`crates/runie-tui/src/ui_actor/mod.rs:566-575` — `deliver_queued` RPC returns after `TurnActor` emitted events, but those events may not have been applied to `AppState` yet when `run_if_queued` is called.

### 4. Query pattern inconsistent

Some actors expose state via `RpcReplyPort` (e.g., `DeliverQueued`), while other code reads `AppState` directly. This inconsistency breaks SSOT.

## Recommended changes

1. Add a true fire-and-forget variant of `DeliverQueued` (no reply port) or require and await the reply.
2. Remove the `Clone` impl for messages containing reply ports.
3. Apply `AppState` projections only after the owning actor has committed and emitted facts.
4. Standardize on actor messages for all authoritative state queries; remove direct `AppState` reads for authoritative data.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Remove unsafe zeroed reply port | `tasks/remove-unsafe-zeroed-reply-port-in-deliverqueued.md` | **new** |
| Remove Clone for messages with reply ports | `tasks/remove-clone-impl-for-messages-with-reply-ports.md` | **new** |
| Serialize projection update after actor commit | `tasks/serialize-projection-update-after-actor-commit.md` | **new** |
