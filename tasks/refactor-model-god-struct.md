# Refactor: Extract AppState into Logical Sub-structs

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture

## Description

`AppState` in `crates/runie-core/src/model.rs` is a 476-line god struct with 40+ fields mixing UI state, business logic, agent state, and presentation concerns. Extract into logical sub-structs:

- **InputState**: `input`, `cursor_pos`, `undo_stack`, `redo_stack`, `history_pos`, `input_flash`, `placeholder`
- **AgentState**: `request_queue`, `message_queue`, `current_request_id`, `turn_started_at`, `turn_active`, `inflight`, `current_tool_name`, `tool_started_at`
- **ViewState**: `scroll`, `elements_cache`, `line_counts`, `total_lines`, `dirty`, `cached_gen`, `message_gen`
- **SessionState**: `messages`, `session_tree`, `session_display_name`, `session_created_at`, `session_updated_at`
- **ConfigState**: `current_provider`, `current_model`, `config_provider`, `config_model`, `keybindings`, `theme_name`, `thinking_level`, `read_only`, `scoped_models`, `scoped_index`
- **CompletionState**: `path_suggestions`, `path_selected`, `at_suggestions`, `at_selected`, `last_at_query`

## Acceptance Criteria

- [ ] `InputState` struct extracted with its methods from `update/input.rs`
- [ ] `AgentState` struct extracted with agent-related fields
- [ ] `ViewState` struct extracted with cache/render-related fields
- [ ] `AppState` becomes a container struct holding sub-structs by value
- [ ] `Snapshot::new_from_state()` updated to copy from sub-structs
- [ ] All field accesses via `app.input.input.clone()` etc. or provide accessor methods
- [ ] All existing tests pass

## Tests

### Layer 1 — State/Logic
- [ ] `test_input_state_undo_redo` — verify undo/redo still works
- [ ] `test_agent_state_queue_operations` — verify queue operations
- [ ] `test_view_state_cache_invalidation` — verify dirty tracking

### Layer 2 — Event Handling
- [ ] All existing event tests pass

### Layer 3 — Rendering
- [ ] `test_snapshot_captures_sub_state` — verify snapshot contains all fields

### Layer 4 — Smoke
- [ ] `smoke_session_lifecycle.sh` — create, interact, save session

## Notes

- This is a pure refactor
- Prefer value types over Arc/Rc unless there's a clear ownership reason
- Keep `AppState` as the single source of truth for `messages_changed()` and `mark_dirty()`
- **Out of scope**: Changing the Snapshot structure (unless needed for correctness)
