# Round 4 — Durable Event Log and Replay

## Findings

### 1. `DurableCoreEvent` is a parallel enum

`crates/runie-core/src/event/durable.rs:333-371` re-declares `MessageSent`, `ToolCalled`, `ToolResult`, etc. This is a SSOT violation: adding an event requires changes in both `Event` and `DurableCoreEvent`.

### 2. Derived values are persisted

`crates/runie-agent/src/turn/mod.rs:128` stores `duration_secs` in `DurableCoreEvent::ToolResult`. Replaying the same raw events on a different clock yields a different duration.

`crates/runie-core/src/update/dispatch.rs:93-104` emits `CompactionTriggered` with computed `ratio`. Persisting derived values makes replay non-deterministic.

### 3. No turn journal phases

Unlike `flue`'s `AgentTurnJournalPhase` (`before_provider` → `provider_started` → `tool_request_recorded` → `committed`), Runie only stores completed events. Crash recovery cannot reconstruct an interrupted turn.

### 4. `SessionChanged` sends whole state

`crates/runie-core/src/actors/session/ractor_session_actor.rs` (inferred) emits `Event::SessionChanged { state: Box<new(state)> }`. Subscribers treat the boxed state as authoritative instead of rebuilding from fine-grained events.

## Recommended changes

1. Collapse `DurableCoreEvent` into the canonical `Event` enum (already a task from pass 1).
2. Remove derived values from durable events; compute them during replay/projection.
3. Introduce turn-journal phases for crash recovery:
   - `TurnStarted`
   - `ProviderCalled`
   - `ToolRequestsRecorded`
   - `StreamChunk` / `ResponseDelta`
   - `TurnCommitted` / `TurnAborted`
4. Replace `SessionChanged` whole-state event with fine-grained events (`MessageAdded`, `PendingEditPushed`, etc.).
5. Add contract tests for `SessionStore` and `TurnActor` idempotency/ordering.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Add turn journal phases | `tasks/add-turn-journal-phases-for-crash-recovery.md` | **new** |
| Remove derived values from durable events | `tasks/remove-derived-values-from-durable-events.md` | **new** |
| Replace whole-state `SessionChanged` | `tasks/replace-wholestate-sessionchanged-with-fine-grained-events.md` | **new** |
| Add contract tests for store/actor | `tasks/add-contract-tests-for-turnactor-and-sessionstore.md` | **new** |
