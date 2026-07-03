# ADR: Everything is Events-Based with SSOT Actors

## Status

Accepted. Supersedes any task or code that relies on direct state mutation, mirrored state, or fire-and-forget side effects outside the actor/event model.

## Context

Runie is an async, multi-actor terminal coding agent. Multiple subsystems (TUI, headless CLI, subagents, MCP servers, provider network calls) need to observe and react to the same runtime facts: configuration loaded, turn started, token streamed, tool called, permission requested, session saved, etc. Without a single source of truth (SSOT) and a single mechanism for change notification (events), the code accumulates:

- Mirrored state (`AgentState` vs `TurnState`, queues in both actor and UI).
- Direct mutation of "someone else's" state from handlers/helpers/tests.
- Fire-and-forget tasks whose outcomes are never observed.
- Race conditions from late subscriptions and polling timeouts.

## Decision

All runtime state is owned by **one actor** and is a **Single Source of Truth (SSOT)**. The only way to change that state is to send the owning actor a message. The only way to observe a change is through **events** published on the `EventBus` (or an actor's outbound channel). Consumers may keep **read-only projections** or **snapshots**, but never authoritative copies.

### Rules

1. **One owner per fact.**
   - `TurnState` is owned by `TurnActor`.
   - `SessionTree` is owned by `SessionActor`.
   - `Config` is owned by `ConfigActor`.
   - `PermissionRegistry` is owned by `PermissionActor`.
   - UI view state is owned by `UiActor`; it is a read-only projection of actor facts, not authoritative.

2. **No direct mutation.**
   - Handlers, tools, subagents, and tests must not call `app_state_mut().field = x` or mutate actor-owned structs directly.
   - Changes flow: caller → actor message → actor state transition → event emitted.

3. **No mirrored authoritative state.**
   - If two places hold the same data, one must be a projection derived from the other via events.
   - Example: `AgentState` must derive from `TurnState`; it may not be mutated independently.

4. **All async work is observed.**
   - A spawned task must have a `JoinHandle` owner, a completion event, or a bounded `JoinSet`.
   - No unbounded `tokio::spawn` fire-and-forget.

5. **Effects are events.**
   - File IO, network calls, provider requests, tool executions, and UI side effects are triggered by events and report outcomes as events.
   - The event vocabulary is canonical; durable events and headless events derive from the same canonical `Event`.

6. **Read-only projections are explicit.**
   - A projection is built from a stream of events or a snapshot request/response.
   - Projections are rebuilt from SSOT on restart/resume, not persisted as authoritative state.

## Consequences

- **Good:** Eliminates dual-state bugs, makes race conditions explicit, enables deterministic replay, simplifies testing (events in, events out), and makes persistence/resumption a side effect of the event log.
- **Bad:** More boilerplate for simple updates; requires careful design of event taxonomy; debugging requires event traces.
- **Migration:** Existing violations (direct `AppState` mutation, `AgentState` mirrors, fire-and-forget provider calls, polling loops) are tracked as `todo` tasks and must be fixed before the corresponding features are considered complete.

## Compliance Checklist for Tasks

Every new or updated task must answer:

- [ ] Which actor owns the authoritative state?
- [ ] Which event(s) cause a state transition?
- [ ] Which event(s) notify observers?
- [ ] Are there any direct mutations or mirrors? If yes, they must be removed.
- [ ] Is async work awaited or cancelled, never dropped?

## References

- `tasks/merge-agentstate-into-turnstate-projection.md`
- `tasks/remove-direct-appstate-mutation-from-tui-handlers.md`
- `tasks/route-app-init-loads-through-actors.md`
- `tasks/offload-agent-turn-from-actor-handler-to-tokio-task.md`
- `tasks/replace-emitfn-mutex-with-async-channel.md`
