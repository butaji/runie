# Refactor: Split AppState.update() God Function

**Status**: done
**Milestone**: R3
**Category**: Core Architecture

## Description

`AppState.update()` in `crates/runie-core/src/update/mod.rs` is a 498-line function handling 60+ event variants in a single match statement. This violates the "Function max: 40 lines" rule and makes the code difficult to test, maintain, and reason about.

Split into logical sub-handlers: `update_input()`, `update_agent()`, `update_dialog()`, `update_palette()`, `update_session_tree()`, `update_settings()`.

## Acceptance Criteria

- [x] `update/input.rs` functions extracted: handlers for Input, Backspace, Newline, Cursor, Delete, Undo/Redo, Paste, Submit
- [x] `agent_event()` handler: handlers for AgentThinking, AgentThoughtDone, AgentToolStart, AgentToolEnd, AgentResponse, AgentTurnComplete, AgentDone, AgentError
- [x] `dialog_toggle_event()` handler: handlers for CommandPalette, ModelSelector, ScopedModels, SessionTree toggles
- [x] `session_event()` handler: handlers for ToggleExpand, ToggleSessionTree, SessionTreeFilterCycle, ForkSession, CloneSession
- [x] Main `update()` match becomes a dispatch table with clear category comments (< 40 lines)
- [x] All existing tests pass
- [x] No regression in functionality

## Tests

### Layer 1 — State/Logic
- [ ] `test_update_input_delegates_to_handler` — verifies event reaches correct sub-handler
- [ ] `test_update_agent_events` — existing agent event tests should still pass
- [ ] `test_update_dialog_events` — existing dialog tests should still pass

### Layer 2 — Event Handling
- [ ] All existing event tests pass without modification
- [ ] `test_input_event_creates_dirty_state` — verify mark_dirty called

### Layer 3 — Rendering
- N/A (pure state refactor)

### Layer 4 — Smoke
- [ ] `smoke_basic_interaction.sh` — verify app still works after refactor

## Notes

- This is a pure refactor — no functional changes
- Preserve existing panic/error behavior
- Consider using a macro to generate the dispatch match for documentation purposes
- **Out of scope**: Changing the Event enum structure

## Related

- `refactor-model-god-struct` — sister refactor for AppState struct
