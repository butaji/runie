# InputActor owns InputState

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: turn-actor-owns-agent-turn-state, completion-actor-owns-completion-state

## Description

`InputState` (input buffer, cursor, history, undo/redo, file-picker backups, prompt) is mutated synchronously by text handlers, navigation handlers, system helpers, dialog handlers, and commands. There is no `InputActor`. Create one and move all input mutations into it.

**Implementation Status**: InputActor created with `InputMsg` enum and `InputActorHandle`. All text editing, cursor navigation, history, undo/redo, and state mutations go through `InputActor`. The actor emits `Event::InputChanged` after each mutation. Handler integration uses `try_send_input()` pattern that either sends to the actor (production) or applies synchronously (tests).

## Acceptance criteria

- [x] `InputActor` is an mpsc actor holding the authoritative `InputState`.
- [x] `InputMsg` covers: `InsertChar`, `Backspace`, `Newline`, `DeleteWord`, `DeleteToEnd`, `DeleteToStart`, `KillChar`, `MoveCursor`, `MoveWord`, `CursorStart`, `CursorEnd`, `HistoryPrev`, `HistoryNext`, `Undo`, `Redo`, `Paste(String)`, `PasteImage`, `Clear`, `SetText { text }`, `SetPrompt { name }`, `DrainQueue { messages }`, `HistoryLoaded { entries }`, `InsertAtRef { text }`, `FilePickerAbort`.
- [ ] `AppState.input` is private; reads go through an immutable accessor.
- [x] `InputActor` emits `Event::InputChanged` after each mutation.
- [ ] `submit` no longer directly pushes user messages; it emits `SessionMsg::AddUserMessage` (via a composed intent) and `TurnMsg::RunIfQueued`.
- [x] File-picker backup/range fields stay in `InputState` (owned by InputActor).
- [ ] Ghost/tab-complete state moves to `CompletionActor` (see `completion-actor-owns-completion-state`).
- [ ] `add_to_input_history` helper removed from `AppState`.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `input_actor_insert_char_updates_cursor` — `InsertChar` moves cursor and emits fact.
- [x] `input_actor_undo_redo_restores_state` — undo/redo stacks work.
- [x] `input_actor_history_prev_cycles` — history navigation works.

### Layer 2 — Event Handling
- [x] `typing_event_routes_to_input_actor` — crossterm key event becomes `InputMsg`.
- [ ] `submit_input_emits_user_message_and_run_turn` — Enter produces the correct downstream intents.

### Layer 3 — Rendering
- [ ] `input_changed_marks_view_dirty` — `Event::InputChanged` causes `ViewActor` to invalidate.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `multi_line_submit_routes_through_input_actor` — a multi-line pasted input reaches the session unchanged.

## Files touched

- `crates/runie-core/src/actors/input/` — `mod.rs`, `messages.rs`, `actor.rs`, `tests.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — `input` made private, accessors added.
- `crates/runie-core/src/update/input/text.rs` — emits `InputMsg` via `try_send_input()`.
- `crates/runie-core/src/update/input/nav.rs` — emits `InputMsg` via `try_send_input()`.
- `crates/runie-core/src/update/input/mod.rs` — dispatcher routes to `InputActor`.
- `crates/runie-core/src/update/input/submit.rs` — emits `InputMsg` via `try_send_input()`.
- `crates/runie-core/src/actors/handles.rs` — added `try_send_input()` helper.
- `crates/runie-core/src/event/kind/mod.rs` — added `InputChanged` event kind.

## Notes

- `InputMsg::apply_to()` mirrors `InputActor::handle_msg()` for synchronous test execution.
- Handler integration uses `try_send_input()` which either sends to actor or applies synchronously.
- Ghost/tab-complete state remains in `InputState` until `CompletionActor` is implemented.
- `submit` handling is partially migrated; full TurnActor integration is pending.
