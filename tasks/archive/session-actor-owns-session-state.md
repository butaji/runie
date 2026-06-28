# SessionActor owns SessionState

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: turn-actor-owns-agent-turn-state

## Description

All `AppState.session` mutations are scattered across input handlers, agent handlers, system helpers, command handlers, and replay helpers. The existing `SessionActor` (`crates/runie-core/src/session_actor.rs`) is **not** an mpsc actor today; it is a broadcast subscriber that replays durable events, persists durable events to JSONL, and maintains the session index/metadata. It does **not** own the in-memory `SessionState`. Promote it to the authoritative owner.

Current violators:
- `model/state/app_state.rs` — `messages_changed`, `reset_session`, `restore_session`.
- `update/input/text.rs` — pushes images and user messages.
- `update/system.rs` — adds system messages (`add_system_msg`), reorders messages (`ensure_turn_complete_last`), removes trust welcome, applies initial trust (pushes trust welcome message).
- `update/session.rs` — forks/clones/navigates session tree, replays messages, queues/delivers messages.
- `update/agent/core.rs` — adds thoughts, tool messages, assistant deltas, turn-complete, errors; reorders/inserts/removes messages (thought replacement, empty-assistant cleanup, compaction-like cleanup).
- `update/agent/mod.rs` — dispatcher that routes `AgentEvent` to the lifecycle functions.
- `update/tools.rs` — pending edit push/drain/clear.
- `model/compaction.rs` and `model/snapshot.rs` — duplicate compaction logic.
- `commands/dsl/handlers/session/*` — `/new`, `/reset`, name, fork.
- `session_replay.rs` — restores session metadata.
- `update/dispatch.rs` — `MessageReplayed`, `SessionLoaded`, `SessionImported`, `BashOutput`.
- `update/command.rs` — load/import fallbacks.

## Acceptance criteria

- [x] `SessionActor` becomes an mpsc actor holding the authoritative `SessionState`. `SessionMsg` does not exist yet; create it from scratch.
- [x] `SessionMsg` enum covers mutation variants: `AddUserMessage`, `AddSystemMessage`, `AddToolMessage`, `UpdateToolMessage`, `AddTurnComplete`, `AddErrorMessage`, `PushPendingEdit`, `DrainPendingEdits`, `ClearPendingEdits`, `ForkAt { index }`, `CloneBranch`, `Reset`.
- [x] `SessionActor` applies each message, updates `session_updated_at`, and emits `Event::SessionChanged`.
- [x] `cargo test --workspace` passes.

## Implementation Notes

The core functionality is implemented:
- `SessionActor` now owns `SessionState` with message/tree/pending_edits fields
- Mutation handlers emit `Event::SessionChanged` on state changes
- Session state mutation methods added to `SessionActorHandle` (`try_*` fire-and-forget)
- `Event::SessionChanged` added to the event enum
- Files split to meet 500-line limit: `mutations.rs`, `tests.rs`

Remaining work (follow-up tasks):
- `AppState.session` should be made private (requires `remove-direct-appstate-mutations`)
- Integration with update/dispatch system to emit intents instead of direct mutations
- Compaction deduplication between `model/compaction.rs` and `model/snapshot.rs`

## Tests

### Layer 1 — State/Logic
- [x] `session_actor_add_user_message_updates_timestamps` — after `AddUserMessage`, `session_updated_at` advances.
- [ ] `session_actor_compact_reduces_messages` — `CompactMessages` keeps the expected tail and inserts summary. (Not yet implemented)
- [x] `session_actor_pending_edit_lifecycle` — push/drain/clear work and update timestamps.

### Layer 2 — Event Handling
- [ ] `submit_input_emits_add_user_message` — pressing Enter sends `SessionMsg::AddUserMessage`, not a direct push.
- [ ] `agent_response_delta_emits_append_assistant_text` — streaming delta routes through `SessionActor`.
- [ ] `session_loaded_replays_via_session_actor` — `SessionStoreActor` result is sent to `SessionActor`.

### Layer 3 — Rendering
- [ ] `session_changed_increments_message_gen` — `Event::SessionChanged` causes `view.message_gen` to advance (via `ViewActor`).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_tool_turn_session_actor_owns_messages` — a full multi-tool turn produces the expected message sequence with no direct `session.messages` writes outside the actor.

## Files touched

- `crates/runie-core/src/actors/session/actor.rs` — holds `SessionState`; mutation handlers in separate module.
- `crates/runie-core/src/actors/session/mutations.rs` — mutation handler implementations.
- `crates/runie-core/src/actors/session/messages.rs` — `SessionMsg` enum with mutation variants.
- `crates/runie-core/src/actors/session/tests.rs` — unit tests for mutation handlers.
- `crates/runie-core/src/actors/handles.rs` — `try_*` fire-and-forget methods on `SessionActorHandle`.
- `crates/runie-core/src/event/variants.rs` — `Event::SessionChanged` variant.
- `crates/runie-core/src/model/state/session.rs` — serde derives for `SessionState`.
- `crates/runie-core/src/edit_preview.rs` — serde derives for `EditPreview`.
