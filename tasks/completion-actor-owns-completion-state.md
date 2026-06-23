# CompletionActor owns CompletionState

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, input-actor-owns-input-state
**Blocks**: none

## Description

Path completion, `@` mention suggestions, ghost text, and tab-completion state are mutated by dialog handlers and legacy at-refs code. There is no `CompletionActor`. Create one and consolidate completion state there.

Current violators:
- `update/path_complete.rs` — toggles/completes path popup.
- `update/dialog/tab_complete.rs` — ghost/tab state (some of this lives in `InputState` currently).
- `update/session.rs` — `abort_queue` clears at-suggestions.
- `update/agent/at_refs.rs` — legacy at-trigger and insertion.
- `update/dialog/form_handler.rs` — `@` ref insertion.

## Acceptance criteria

- [ ] `CompletionActor` is an mpsc actor holding the authoritative `CompletionState` plus ghost/tab fields (move ghost/tab fields from `InputState` into `CompletionState`).
- [ ] `CompletionMsg` covers: `TogglePathCompletion`, `PathCompletionUp`, `PathCompletionDown`, `PathCompletionSelect`, `PathCompletionClose`, `TabComplete`, `AcceptGhost`, `ClearGhost`, `AtSuggestionsChanged { suggestions }`, `InsertAtSuggestion { index }`, `ClearAtRef`.
- [ ] `AppState.completion` is private; reads go through an immutable accessor.
- [ ] `CompletionActor` emits `Event::CompletionChanged` after mutations.
- [ ] Path completion no longer mutates `input` directly; selecting an entry sends `InputMsg::InsertText { text }`.
- [ ] Legacy `update/agent/at_refs.rs` is deleted or folded into `CompletionActor`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `completion_actor_path_select_inserts_text` — selecting a path suggestion produces the right input text.
- [ ] `completion_actor_at_suggestions_filter` — `@` suggestions are filtered and ranked.

### Layer 2 — Event Handling
- [ ] `tab_key_sends_tab_complete_intent` — Tab routes to `CompletionActor`.
- [ ] `path_completion_select_routes_to_completion_actor` — Enter in path popup sends `PathCompletionSelect`.

### Layer 3 — Rendering
- [ ] `path_completion_popup_renders` — `CompletionChanged` causes the popup to render.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/completion/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/state/input.rs` — move ghost/tab fields to `CompletionState`.
- `crates/runie-core/src/state/completion.rs` — add ghost/tab fields.
- `crates/runie-core/src/model/state/app_state.rs` — private `completion`.
- `crates/runie-core/src/update/path_complete.rs` — emit `CompletionMsg`.
- `crates/runie-core/src/update/dialog/tab_complete.rs` — emit `CompletionMsg`.
- `crates/runie-core/src/update/agent/at_refs.rs` — delete or fold into actor.
- `crates/runie-core/src/update/session.rs` — `abort_queue` sends `CompletionMsg::ClearAtRef`.
- `crates/runie-core/src/update/dialog/form_handler.rs` — `@` insertion emits `CompletionMsg`.

## Notes

- Coordinate with `input-actor-owns-input-state` on the boundary: `CompletionActor` decides *what* to insert; `InputActor` performs the insertion.
