# Decompose AppState God-Object Into Focused State Structs

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

`AppState` is a god-object. The struct has 33 top-level fields (lines
202-258 in `crates/runie-core/src/model.rs`), of which only 6 have
been moved into the inner `state::SessionState/InputState/AgentState/
ViewState/ConfigState/CompletionState` structs. The rest
(`streaming`, `next_id`, `animation_frame`, `current_action`,
`last_assistant_index`, `thought_seq`, `input_history`,
`cached_palette_items`, `cached_palette_filter`, `cached_model_items`,
`cached_model_filter`, `transient_message/until/level`, `git_info`,
`cwd_name`, `image_attachments`, `pending_edits`, `skills`,
`prompts`, `current_prompt`, `recent_models`, `telemetry`,
`all_collapsed`, `dialog_back_stack`, `open_dialog`, `login_flow`,
`should_quit`, `registry`, `steering_mode`, `follow_up_mode`,
`thinking_started_at`, `intermediate_step_count`) live on the
outer `AppState`. The mid-refactor state is what caused the merge
conflict in `impl Default for AppState` (which has been resolved in
`77a605c3` — the `Default` impl is now ~50 lines with all fields
listed).

The current `AppState` field list (from `crates/runie-core/src/model.rs:202-258`):

```rust
pub struct AppState {
    // 6 inner structs (already factored)
    pub session: SessionState,
    pub input: InputState,
    pub agent: AgentState,
    pub view: ViewState,
    pub config: ConfigState,
    pub completion: CompletionState,

    // 27 loose fields (still on AppState)
    pub streaming: bool,
    pub thinking_started_at: Option<Instant>,
    pub steering_mode: DeliveryMode,
    pub follow_up_mode: DeliveryMode,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub animation_frame: u32,
    pub current_action: Option<String>,
    pub registry: CommandRegistry,
    pub should_quit: bool,
    pub open_dialog: Option<DialogState>,
    pub dialog_back_stack: Vec<DialogState>,
    pub login_flow: Option<LoginFlowState>,
    pub recent_models: Vec<String>,
    pub pending_edits: Vec<EditPreview>,
    pub skills: Vec<Skill>,
    pub telemetry: Telemetry,
    pub prompts: Vec<PromptTemplate>,
    pub current_prompt: String,
    pub image_attachments: Vec<String>,
    pub all_collapsed: bool,
    pub(crate) last_assistant_index: Option<usize>,
    pub(crate) thought_seq: u64,
    pub(crate) input_history: Vec<String>,
    pub transient_message: Option<String>,
    pub transient_until: Option<Instant>,
    pub transient_level: Option<TransientLevel>,
    pub git_info: Option<GitInfo>,
    pub cwd_name: String,
    // + 4 private cache fields (palette, model selector)
    cached_palette_items: Vec<...>,
    cached_palette_filter: Option<String>,
    cached_model_items: Vec<...>,
    cached_model_filter: Option<String>,
}
```

That's 33 pub + 4 private = **37 fields** on the outer struct.

## Acceptance Criteria

- [ ] `AppState` has exactly 6 fields: `session`, `input`, `agent`,
  `view`, `config`, `completion` (matching the existing inner
  structs) **plus** the documented singletons below
- [ ] All 33 loose fields are moved into the appropriate inner
  struct. Proposed distribution:
  - **`AgentState`** gets: `request_queue`, `message_queue`,
    `current_request_id`, `turn_started_at`, `turn_active`,
    `inflight`, `current_tool_name`, `tool_started_at`, `tokens_*`,
    `speed_*` (already there). **Add**: `next_id`,
    `intermediate_step_count`, `current_action`, `thought_seq`,
    `last_assistant_index`, `streaming`, `thinking_started_at`
  - **`ViewState`** gets: animation/scroll/cache fields (already
    there). **Add**: `cached_palette_items`, `cached_palette_filter`,
    `cached_model_items`, `cached_model_filter`, `all_collapsed`,
    `animation_frame`
  - **`ConfigState`** gets: existing config fields (already
    there). **Add**: `scoped_models`, `recent_models` (already
    there), `current_prompt`, `pending_edits`, `image_attachments`,
    `telemetry`, `steering_mode`, `follow_up_mode`
  - **`CompletionState`** (already has some). **Add**:
    `last_at_query` and the `tab_complete_*` fields from
    `InputState` (which currently own them but logically belong
    here)
- [ ] New top-level fields on `AppState` (singletons that don't
  fit any of the 6): `should_quit: bool`, `open_dialog:
  Option<DialogState>`, `dialog_back_stack: Vec<DialogState>`,
  `login_flow: Option<LoginFlowState>`, `registry: CommandRegistry`,
  `skills: Vec<Skill>`, `prompts: Vec<PromptTemplate>`,
  `transient_message/until/level: Transient*`, `git_info`,
  `cwd_name`, `input_history: Vec<String>`
- [ ] All call sites that access `state.field` are updated to
  `state.session.field` / `state.input.field` / etc.
- [ ] `impl Default for AppState` is at most 30 lines (one line
  per field, with delegated `..Default::default()`)
- [ ] No function takes a `&mut AppState` and immediately rebinds
  one of these sub-fields — the inner struct is the unit of
  mutation

## Tests

### Layer 1 — State/Logic
- [ ] `test_appstate_default_has_no_loose_fields` — iterates
  `AppState::default()` and asserts no top-level field is a `Vec`,
  `Option`, or state-bearing primitive outside the 6 inner structs
  and the documented singletons
- [ ] `test_inner_structs_are_default` — each of `SessionState`,
  `InputState`, `AgentState`, `ViewState`, `ConfigState`,
  `CompletionState` derives `Default` and round-trips through
  serialization
- [ ] `test_moved_field_access_pattern` — every public method on
  `AppState` that previously read/wrote a now-moved field is
  updated; the existing test suite catches the regressions

### Layer 2 — Event Handling
- [ ] All 595+ existing `#[test]` annotations in
  `crates/runie-core/src/tests/` still pass
- [ ] `cargo test -p runie-core --lib update::` passes (event
  dispatch covers all sub-state mutations)

## Notes

**Migration strategy** — do this in three commits, each
independently buildable:

1. **Add fields to inner structs** (no removal of outer fields).
   E.g. `agent.next_id: u64`, `view.cached_palette_items: Vec<…>`.
   All call-sites continue to work because the outer field still
   exists.

2. **Redirect read-sites** to the inner struct. E.g.
   `state.next_id` → `state.agent.next_id`. Compile errors are the
   breadcrumb.

3. **Remove outer fields** once nothing reads them. `impl Default`
   shrinks to the final shape.

**Singletons that stay on `AppState`:**

- `should_quit: bool` — the main loop reads this; it's a control
  flag, not state
- `open_dialog`, `dialog_back_stack` — UI overlay state, not a
  "session" concern
- `login_flow: Option<LoginFlowState>` — the login flow is an
  overlay, similar to dialog
- `registry: CommandRegistry` — the command table is loaded once
  and is a singleton
- `skills: Vec<Skill>`, `prompts: Vec<PromptTemplate>` — loaded
  from disk, treated as immutable per session
- `transient_message/until/level` — UI notification, lives until
  cleared
- `git_info`, `cwd_name` — detected once on startup, immutable
- `input_history: Vec<String>` — persistent across sessions, not
  part of `InputState` per session

**`CompletionState` is the only new struct needed.** Its current
fields (`path_suggestions`, `path_selected`, `at_suggestions`,
`at_selected`) are correct. The `tab_complete_*` fields currently
in `InputState` should move here.

**Out of scope:**
- Changing the public API of `AppState` (callers like
  `runie-term`, `runie-tui` are updated as a consequence, not
  redesigned)
- Splitting `AppState::snapshot()` into per-state snapshot builders
  (separate refactor; see `snapshot-dead-code`)

**Verification:**
```bash
# Should be zero
git grep -nE 'state\.(streaming|next_id|animation_frame|current_action|thought_seq|last_assistant_index|cached_palette|cached_model|transient_message|transient_until|transient_level|git_info|cwd_name|image_attachments|pending_edits|skills|prompts|current_prompt|recent_models|input_history|telemetry|all_collapsed|dialog_back_stack|open_dialog|login_flow|should_quit|registry|steering_mode|follow_up_mode|thinking_started_at|intermediate_step_count)\b' -- 'crates/runie-core/src/'

cargo test -p runie-core --lib
```
