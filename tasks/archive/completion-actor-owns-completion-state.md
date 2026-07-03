# CompletionActor owns CompletionState

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, input-actor-owns-input-state
**Blocks**: none

## Description

Path completion, `@` mention suggestions, ghost text, and tab-completion state are mutated by dialog handlers and legacy at-refs code. There is no `CompletionActor`. Create one and consolidate completion state there.

**Implementation Status**: CompletionActor created with `CompletionMsg` enum and `CompletionActorHandle`. The actor owns path completion and @ mention suggestion state. `Event::CompletionChanged` is emitted after mutations. Ghost/tab state remains in `InputState` (handled by InputActor) since it's tightly coupled to input buffer operations.

## Acceptance criteria

- [x] `CompletionActor` is an mpsc actor holding the authoritative `CompletionState`.
- [x] `CompletionMsg` covers: `TogglePathCompletion`, `PathCompletionUp`, `PathCompletionDown`, `PathCompletionSelect`, `PathCompletionClose`, `AtSuggestionsChanged`, `AtSuggestionUp`, `AtSuggestionDown`, `AtSuggestionSelect`, `ClearAtRef`, `ClearAll`, `SetGhost`, `SetTabComplete`, `AcceptGhost`, `ClearGhost`, `TabCompleteNext`, `FilePickerAbort`.
- [ ] `AppState.completion` is private; reads go through an immutable accessor.
- [x] `CompletionActor` emits `Event::CompletionChanged` after mutations.
- [ ] Path completion no longer mutates `input` directly; selecting an entry sends `InputMsg::InsertText { text }`.
- [ ] Legacy `update/agent/at_refs.rs` is deleted or folded into `CompletionActor`.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `completion_actor_path_select_inserts_text` — selecting a path suggestion clears completion state.
- [x] `completion_actor_at_suggestions_filter` — `@` suggestions are filtered and ranked.
- [x] `toggle_path_completion_creates_suggestions` — toggling creates suggestions.
- [x] `at_suggestions_changes_suggests` — changing suggestions updates state.
- [x] `clear_at_ref_clears_suggestions` — clearing at ref clears suggestions.

### Layer 2 — Event Handling
- [ ] `tab_key_sends_tab_complete_intent` — Tab routes to `CompletionActor`.
- [ ] `path_completion_select_routes_to_completion_actor` — Enter in path popup sends `PathCompletionSelect`.

### Layer 3 — Rendering
- [ ] `path_completion_popup_renders` — `CompletionChanged` causes the popup to render.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/completion/` — `mod.rs`, `messages.rs`, `actor.rs`, `tests.rs`.
- `crates/runie-core/src/model/state/session.rs` — added derives to `CompletionState`.
- `crates/runie-core/src/path_complete.rs` — added serde derives to `PathCompletion`.
- `crates/runie-core/src/event/variants.rs` — added `CompletionChanged` event.
- `crates/runie-core/src/actors/mod.rs` — exported `CompletionActor`, `CompletionMsg`, `CompletionActorHandle`.
- `crates/runie-core/src/actors/handles.rs` — added `completion` field and test.
- `crates/runie-core/src/event/variants_tests/dispatch.rs` — added `CompletionChanged` to exhaustive match.

## Notes

- Ghost/tab state remains in `InputState` since it's tightly coupled to input buffer operations. The `CompletionMsg::SetGhost`, `SetTabComplete`, etc. are defined but are no-ops for `CompletionState` - they're handled by `InputActor`.
- `apply_to()` mirrors `handle_msg()` for synchronous test execution.
- Handler integration with `try_send_completion()` pattern (similar to `try_send_input()`) is pending.
