# InputActor owns InputState

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: turn-actor-owns-agent-turn-state, completion-actor-owns-completion-state

## Description

`InputState` (input buffer, cursor, history, undo/redo, file-picker backups, prompt) is mutated synchronously by text handlers, navigation handlers, system helpers, dialog handlers, and commands. There is no `InputActor`. Create one and move all input mutations into it.

Current violators:
- `update/input/text.rs` — every text edit, history, undo/redo, paste, submit.
- `update/input/nav.rs` — cursor movement and vim-nav state changes.
- `update/input/mod.rs` — escape handling, mouse, resize (resize goes to ViewActor).
- `update/system.rs` — `stop_turn` drains message queue to input; `handle_editor_done`; `handle_quit_event` clears input.
- `update/session.rs` — `queue_follow_up`, `abort_queue`, `dequeue`.
- `update/dispatch.rs` — `HistoryLoaded` updates `input_history`.
- `update/dialog/router.rs` — abort clears input.
- `update/dialog/form_handler.rs` — `@` ref insertion.
- `update/dialog/open.rs` — file picker backup fields.
- `update/dialog/tab_complete.rs` — ghost/tab state (or move to CompletionActor).
- `commands/dsl/handlers/session/mod.rs` — `/new` clears input.
- `commands/dsl/handlers/system.rs` — `/prompt` sets `current_prompt`.
- `model/state/app_state.rs` — `add_to_input_history` helper.

## Acceptance criteria

- [ ] `InputActor` is an mpsc actor holding the authoritative `InputState`.
- [ ] `InputMsg` covers: `InsertChar`, `Backspace`, `Newline`, `DeleteWord`, `DeleteToEnd`, `DeleteToStart`, `KillChar`, `MoveCursor { direction }`, `MoveWord { direction }`, `MoveToLineStart`, `MoveToLineEnd`, `HistoryPrev`, `HistoryNext`, `Undo`, `Redo`, `Paste(String)`, `PasteImage`, `Submit`, `Clear`, `SetText { text }`, `SetPrompt { name }`, `DrainQueue { messages }`, `HistoryLoaded { entries }`, `InsertAtRef { text }`, `FilePickerAbort`.
- [ ] `AppState.input` is private; reads go through an immutable accessor.
- [ ] `InputActor` emits `Event::InputChanged` after each mutation.
- [ ] `submit` no longer directly pushes user messages; it emits `SessionMsg::AddUserMessage` (via a composed intent) and `TurnMsg::RunIfQueued`.
- [ ] File-picker backup/range fields either stay in `InputState` (owned by InputActor) or move to `CompletionState` if they are completion concerns.
- [ ] Ghost/tab-complete state moves to `CompletionActor` (see `completion-actor-owns-completion-state`).
- [ ] `add_to_input_history` helper removed from `AppState`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `input_actor_insert_char_updates_cursor` — `InsertChar` moves cursor and emits fact.
- [ ] `input_actor_undo_redo_restores_state` — undo/redo stacks work.
- [ ] `input_actor_history_prev_cycles` — history navigation works.

### Layer 2 — Event Handling
- [ ] `typing_event_routes_to_input_actor` — crossterm key event becomes `InputMsg`.
- [ ] `submit_input_emits_user_message_and_run_turn` — Enter produces the correct downstream intents.

### Layer 3 — Rendering
- [ ] `input_changed_marks_view_dirty` — `Event::InputChanged` causes `ViewActor` to invalidate.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `multi_line_submit_routes_through_input_actor` — a multi-line pasted input reaches the session unchanged.

## Files touched

- `crates/runie-core/src/actors/input/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — private `input`, remove `add_to_input_history`.
- `crates/runie-core/src/update/input/text.rs` — emit `InputMsg`.
- `crates/runie-core/src/update/input/nav.rs` — emit `InputMsg`.
- `crates/runie-core/src/update/input/mod.rs` — dispatcher sends `InputMsg`.
- `crates/runie-core/src/update/system.rs` — `stop_turn`, `handle_editor_done`, quit cleanup emit intents.
- `crates/runie-core/src/update/session.rs` — queue follow-up/abort/dequeue emit `InputMsg`.
- `crates/runie-core/src/update/dialog/router.rs`, `form_handler.rs`, `open.rs`, `tab_complete.rs` — emit intents.
- `crates/runie-core/src/update/dispatch.rs` — `HistoryLoaded` routes to `InputActor`.
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — `/new` emits `InputMsg::Clear`.
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/prompt` emits `InputMsg::SetPrompt`.

## Notes

- Coordinate with `turn-actor-owns-agent-turn-state` on the exact split between `InputMsg::Submit` and `TurnMsg::RunIfQueued`.
- Cursor math (graphemes, scroll) can remain pure helpers inside the actor module; the actor owns the state but can use pure functions.
