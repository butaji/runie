# Decompose AppState God-Object Into Focused State Structs

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

`AppState` is a god-object. The struct has 40+ top-level fields, of which
only ~12 have been moved into the inner `state::SessionState/InputState/
AgentState/ViewState/ConfigState/CompletionState` structs. The rest
(`streaming`, `next_id`, `animation_frame`, `current_action`,
`thought_seq`, `last_assistant_index`, `cached_palette_items`,
`cached_model_items`, `transient_message/until/level`, `git_info`,
`cwd_name`, `image_attachments`, `pending_edits`, `skills`, `prompts`,
`current_prompt`, `recent_models`, `input_history`, `telemetry`,
`all_collapsed`, `dialog_back_stack`, `open_dialog`, `login_flow`,
`should_quit`, `registry`, `steering_mode`, `follow_up_mode`) live on the
outer `AppState`. The mid-refactor state is what caused the merge
conflict in `impl Default for AppState` (lines 260-709 of `model.rs`).

## Acceptance Criteria

- [ ] `AppState` has exactly 6 fields: `session`, `input`, `agent`, `view`, `config`, `completion` (matching the existing inner structs)
- [ ] All 40+ loose fields are moved into the appropriate inner struct. Specifically:
  - `SessionState` gets: `messages`, `session_tree`, `session_display_name`, `session_created_at`, `session_updated_at` (already done — no change)
  - `InputState` gets: `input`, `cursor_pos`, `undo_stack`, `redo_stack`, `history_pos`, `input_flash`, `placeholder`, `ghost_completion`, `tab_complete_*`, `input_scroll` (already done — no change)
  - `AgentState` gets: `request_queue`, `message_queue`, `current_request_id`, `turn_started_at`, `turn_active`, `inflight`, `current_tool_name`, `tool_started_at`, `tokens_in/out`, `speed_tps`, `speed_window`, animation/easing fields (already done — no change). **Add**: `next_id`, `intermediate_step_count`, `current_action`, `thought_seq`, `last_assistant_index`, `streaming`
  - `ViewState` gets: animation/scroll/cache fields (already done — no change). **Add**: `cached_palette_items`, `cached_palette_filter`, `cached_model_items`, `cached_model_filter`, `all_collapsed`
  - `ConfigState` gets: existing config fields (no change). **Add**: `scoped_models`, `recent_models` (already there), `current_prompt`, `pending_edits`, `image_attachments`, `telemetry`
  - **`CompletionState`** (new) gets: `path_suggestions`, `path_selected`, `at_suggestions`, `at_selected`, `last_at_query`
- [ ] New top-level fields: `AppState` also owns the singleton singletons that don't fit any of the 6: `registry: CommandRegistry`, `should_quit: bool`, `open_dialog: Option<DialogState>`, `dialog_back_stack: Vec<DialogState>`, `login_flow: Option<LoginFlowState>`, `skills: Vec<Skill>`, `prompts: Vec<PromptTemplate>`, `transient_message/until/level: Transient*`, `git_info`, `cwd_name`, `input_history: Vec<String>`, `steering_mode`, `follow_up_mode`
- [ ] All call sites that access `state.field` are updated to `state.session.field` / `state.input.field` / etc.
- [ ] `impl Default for AppState` is at most 30 lines (one line per field, with delegated `..Default::default()`)
- [ ] No function takes a `&mut AppState` and immediately rebinds one of these sub-fields — the inner struct is the unit of mutation

## Tests

### Layer 1 — State/Logic
- [ ] `test_appstate_default_has_no_loose_fields` — iterates `AppState::default()` and asserts no top-level field is a `Vec`, `Option`, or state-bearing primitive outside the 6 inner structs and the documented singletons
- [ ] `test_inner_structs_are_default` — each of `SessionState`, `InputState`, `AgentState`, `ViewState`, `ConfigState`, `CompletionState` derives `Default` and round-trips through serialization
- [ ] `test_moved_field_access_pattern` — every public method on `AppState` that previously read/wrote a now-moved field is updated; the existing test suite catches the regressions

### Layer 2 — Event Handling
- [ ] All 595+ existing `#[test]` annotations in `crates/runie-core/src/tests/` still pass
- [ ] `cargo test -p runie-core --lib update::` passes (event dispatch covers all sub-state mutations)

## Notes

**Migration strategy** — do this in three commits, each independently
buildable:

1. **Add fields to inner structs** (no removal of outer fields). E.g.
   `agent.next_id: u64`, `view.cached_palette_items: Vec<…>`. All
   call-sites continue to work because the outer field still exists.

2. **Redirect read-sites** to the inner struct. E.g.
   `state.next_id` → `state.agent.next_id`. Compile errors are the
   breadcrumb.

3. **Remove outer fields** once nothing reads them. `impl Default` shrinks
   to the final shape.

**Singletons that stay on `AppState`:**

- `should_quit: bool` — the main loop reads this; it's a control flag, not state
- `open_dialog`, `dialog_back_stack` — UI overlay state, not a "session" concern
- `login_flow: Option<LoginFlowState>` — the login flow is an overlay, similar to dialog
- `registry: CommandRegistry` — the command table is loaded once and is a singleton
- `skills: Vec<Skill>`, `prompts: Vec<PromptTemplate>` — loaded from disk, treated as immutable per session
- `transient_message/until/level` — UI notification, lives until cleared
- `git_info`, `cwd_name` — detected once on startup, immutable
- `input_history: Vec<String>` — persistent across sessions, not part of `InputState` per session
- `steering_mode`, `follow_up_mode` — user preferences, conceptually part of config

**`CompletionState` is the only new struct needed.** Its fields
(`path_suggestions`, `path_selected`, `at_suggestions`, `at_selected`,
`last_at_query`) are currently sprinkled between `AppState` and
`InputState` (the `tab_complete_*` fields in `InputState` actually
belong in `CompletionState`).

**Out of scope:**
- Changing the public API of `AppState` (callers like `runie-term`,
  `runie-tui` are updated as a consequence, not redesigned)
- Splitting `AppState::snapshot()` into per-state snapshot builders (separate refactor)

**Verification:**
```bash
# Should be zero
git grep -nE 'state\.(streaming|next_id|animation_frame|current_action|thought_seq|last_assistant_index|cached_palette|cached_model|transient_message|transient_until|transient_level|git_info|cwd_name|image_attachments|pending_edits|skills|prompts|current_prompt|recent_models|input_history|telemetry|all_collapsed|dialog_back_stack|open_dialog|login_flow|should_quit|registry|steering_mode|follow_up_mode)\b' -- 'crates/runie-core/src/'

cargo test -p runie-core --lib
```
