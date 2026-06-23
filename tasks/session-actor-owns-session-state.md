# SessionActor owns SessionState

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: turn-actor-owns-agent-turn-state

## Description

All `AppState.session` mutations are scattered across input handlers, agent handlers, system helpers, command handlers, and replay helpers. The existing `SessionActor` (`crates/runie-core/src/session_actor.rs`) only appends durable events; it does not own the in-memory `SessionState`. Promote it to the authoritative owner.

Current violators:
- `model/state/app_state.rs` — `messages_changed`, `reset_session`, `restore_session`.
- `update/input/text.rs` — pushes images and user messages.
- `update/system.rs` — adds system messages, reorders messages, removes trust welcome, applies initial trust.
- `update/session.rs` — forks/clones/navigates session tree, replays messages, queues/delivers messages.
- `update/agent/core.rs` — adds thoughts, tool messages, assistant deltas, turn-complete, errors; reorders messages.
- `update/tools.rs` — pending edit push/drain/clear.
- `model/compaction.rs` and `model/snapshot.rs` — duplicate compaction logic.
- `commands/dsl/handlers/session/*` — `/new`, `/reset`, name, fork.
- `session_replay.rs` — restores session metadata.
- `update/dispatch.rs` — `MessageReplayed`, `SessionLoaded`, `SessionImported`, `BashOutput`.
- `update/command.rs` — load/import fallbacks.

## Acceptance criteria

- [ ] `SessionActor` becomes an mpsc actor holding the authoritative `SessionState`.
- [ ] `SessionMsg` enum covers every mutation: `AddUserMessage`, `AttachImage`, `AddSystemMessage`, `RemoveSystemMessage { id }`, `AppendAssistantText`, `SetAssistantMessage`, `InsertThought`, `AddToolMessage`, `UpdateToolMessage`, `CompleteTurn`, `FinishTurn`, `AddErrorMessage`, `PushPendingEdit`, `DrainPendingEdits`, `ClearPendingEdits`, `ForkAt { index }`, `CloneBranch`, `NavigateTo { path }`, `RenameSession { name }`, `CompactMessages { keep_tokens }`, `ResetSession`, `ReplayEvents { events }`, `ImportSession { session }`.
- [ ] `AppState.session` is private; reads go through an immutable accessor.
- [ ] `SessionActor` applies each message, updates `session_updated_at`, and emits `Event::SessionChanged` (or domain-specific facts such as `UserMessageAppended`).
- [ ] `messages_changed()` helper is removed from `AppState`; timestamp bumping lives in `SessionActor`.
- [ ] `model/compaction.rs` and `model/snapshot.rs` deduplicated; compaction is a `SessionActor` helper.
- [ ] `SessionStoreActor` stays a file-IO actor only: it returns durable events/metadata, and `SessionActor` applies them.
- [ ] `PersistenceActor` stays trust/history only; it does not own chat messages.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `session_actor_add_user_message_updates_timestamps` — after `AddUserMessage`, `session_updated_at` advances.
- [ ] `session_actor_compact_reduces_messages` — `CompactMessages` keeps the expected tail and inserts summary.
- [ ] `session_actor_pending_edit_lifecycle` — push/drain/clear work and update timestamps.

### Layer 2 — Event Handling
- [ ] `submit_input_emits_add_user_message` — pressing Enter sends `SessionMsg::AddUserMessage`, not a direct push.
- [ ] `agent_response_delta_emits_append_assistant_text` — streaming delta routes through `SessionActor`.
- [ ] `session_loaded_replays_via_session_actor` — `SessionStoreActor` result is sent to `SessionActor`.

### Layer 3 — Rendering
- [ ] `session_changed_increments_message_gen` — `Event::SessionChanged` causes `view.message_gen` to advance (via `ViewActor`).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_tool_turn_session_actor_owns_messages` — a full multi-tool turn produces the expected message sequence with no direct `session.messages` writes outside the actor.

## Files touched

- `crates/runie-core/src/session_actor.rs` — promote to mpsc actor; add `SessionState` ownership.
- `crates/runie-core/src/actors/session_store/actor.rs` — return events/metadata instead of mutating state.
- `crates/runie-core/src/model/state/app_state.rs` — private `session`, remove `messages_changed`.
- `crates/runie-core/src/update/input/text.rs` — emit `SessionMsg` for image attach and submit.
- `crates/runie-core/src/update/system.rs` — trust/system-message helpers emit intents.
- `crates/runie-core/src/update/session.rs` — queue/tree/session-name helpers emit intents.
- `crates/runie-core/src/update/agent/core.rs` — agent lifecycle emits `SessionMsg` facts.
- `crates/runie-core/src/update/tools.rs` — pending edits emit intents.
- `crates/runie-core/src/update/dispatch.rs` — `SessionLoaded`/`Imported`/`MessageReplayed` route to `SessionActor`.
- `crates/runie-core/src/update/command.rs` — load/import use `SessionActor`.
- `crates/runie-core/src/commands/dsl/handlers/session/*` — `/new`, `/reset`, `/name`, `/fork` emit intents.
- `crates/runie-core/src/session_replay.rs` — becomes internal to `SessionActor`.
- `crates/runie-core/src/model/compaction.rs`, `model/snapshot.rs` — delete one, keep the other as actor helper.

## Notes

- `SessionActor` should keep durable JSONL append behavior, but in-memory state and durable log stay in the same actor.
- Coordinate with `turn-actor-owns-agent-turn-state`: `TurnActor` decides *when* to append; `SessionActor` decides *how* to append.
