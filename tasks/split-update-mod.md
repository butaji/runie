# Split crates/runie-core/src/update/mod.rs Into Focused Files

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1
**Depends on**: resolve-merge-conflicts, fix-broken-references, deduplicate-login-flow

## Description

`crates/runie-core/src/update/mod.rs` is 1926 lines — 1426 over the
project's 500-line cap. It contains:

- The main `AppState::update` dispatcher (lines 60-111)
- 8 free `fn *_event` dispatchers: `transient_event`, `system_event`,
  `scroll_event`, `input_event`, `agent_event`, `handle_history_prev`,
  `handle_history_next`, `model_config_event` (the `*_event` pattern is
  inconsistent — some are methods, some are free functions)
- 30+ `impl AppState` methods (model switching, theme switching, thinking
  level, scoped models, dialog handling, palette, model selector, login
  flow helpers, form panel, command results, edit approval, etc.)
- The `FormAction` enum
- The `partition_model_items` helper
- A `fn form_panel_action` (70+ lines, 5+ branches)
- A `fn form_panel_edit_char`
- A `fn form_build_submit`
- A `fn try_activate_panel`
- A `fn handle_panel_action`
- A `fn panel_toggle_item`
- A `fn panel_cycle_item`
- A `fn apply_panel_setting`
- A `fn process_command_result`
- A `fn push_dialog_to_back_stack`
- A `fn open_at_file_picker`
- A `fn insert_at_ref`
- A `fn toggle_expand_all`
- A `fn apply_form_action`
- A `fn form_dialog_event`

This is 3+ complete subsystems (event dispatch, dialog system, form
panels) stuffed into one file. The merge conflict region (lines 60-440)
exacerbates the problem because it duplicates logic with the sibling
files in `update/` (e.g. `update/login_flow.rs`, `update/palette.rs`,
`update/model_selector.rs`, `update/scoped_models.rs`).

## Acceptance Criteria

- [ ] `crates/runie-core/src/update/mod.rs` is ≤ 500 lines
- [ ] The main `AppState::update` dispatcher stays in `mod.rs` but is ≤ 40 lines (just the category match)
- [ ] The login-flow handlers (currently split between `update/mod.rs` and `update/login_flow.rs`) are consolidated in `update/login_flow.rs` only — see `deduplicate-login-flow` task
- [ ] Dialog system methods move to a new `update/dialog.rs` (or fold into existing `update/dialog_actions.rs` and `update/dialog_open.rs`)
- [ ] Form-panel logic (`form_panel_action`, `form_panel_edit_char`, `form_build_submit`, `form_dialog_event`, `apply_form_action`, `FormAction` enum) moves to a new `update/form.rs`
- [ ] The `partition_model_items` helper and related model-selector glue moves to `update/model_selector.rs` (which already exists)
- [ ] Free function dispatchers (`transient_event`, `system_event`, `scroll_event`, `input_event`, `agent_event`, `handle_history_*`, `model_config_event`) stay in `mod.rs` because they're called from the dispatcher
- [ ] `update/scoped_models.rs` keeps all scoped-models methods; the duplicates in `mod.rs` are removed
- [ ] No function in the split files exceeds 40 lines
- [ ] No file in `crates/runie-core/src/update/` exceeds 500 lines

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds after the split
- [ ] `cargo build --workspace --tests` succeeds (the merge resolution should have produced a buildable state; the split must preserve that)
- [ ] `cargo test -p runie-core --lib update::mod` passes (the dispatcher still routes events)
- [ ] `cargo test -p runie-core --lib update::dialog` passes (dialog handlers moved to a new file)
- [ ] `cargo test -p runie-core --lib update::form` passes (form panel tests move with the form code)

### Layer 2 — Event Handling
- [ ] All 595+ existing `#[test]` annotations in `crates/runie-core/src/tests/` still pass
- [ ] `cargo test -p runie-core --lib tests::dialog_theme_switch` passes (covers dialog + theme switching)
- [ ] `cargo test -p runie-core --lib tests::form_dialog` passes (covers form panel logic)
- [ ] `cargo test -p runie-core --lib tests::model_selector` passes (covers model selector)

## Notes

**Proposed file layout after the split:**

```
crates/runie-core/src/update/
├── mod.rs              (≤ 300 lines: dispatcher + free event handlers)
├── at_refs.rs          (existing)
├── agent.rs            (existing — agent_event stays here, free function)
├── bash.rs             (existing)
├── control.rs          (existing)
├── dialog.rs           (NEW — open_dialog, update_dialog, process_command_result, push_dialog_to_back_stack, open_at_file_picker, insert_at_ref)
├── dialog_actions.rs   (existing — keep as-is)
├── dialog_open.rs      (existing — keep as-is)
├── dialog_update.rs    (existing — keep as-is, used by dialog.rs)
├── edit.rs             (existing)
├── edit_approval.rs    (existing)
├── form.rs             (NEW — FormAction enum, form_panel_action, form_panel_edit_char, form_build_submit, form_dialog_event, apply_form_action, update_form_panel)
├── input.rs            (existing — input_event stays here, free function)
├── input_scroll.rs     (existing)
├── input_text.rs       (existing)
├── line_nav.rs         (existing)
├── login_flow.rs       (existing — keep ALL login logic here, per deduplicate-login-flow)
├── model_selector.rs   (existing — move partition_model_items + handle here)
├── palette.rs          (existing)
├── path_complete.rs    (existing)
├── queue.rs            (existing)
├── scoped_models.rs    (existing — keep ALL scoped-model logic here)
├── session.rs          (existing)
├── settings_dialog.rs  (existing)
├── system_actions.rs   (existing — system_event stays here, free function)
└── tab_complete.rs     (existing)
```

**The `free function vs method` inconsistency** — the merge left some
dispatchers as `fn agent_event(state: &mut AppState, event: Event)` and
others as `AppState::agent_event(&mut self, event: Event)`. The
refactor commit `7c5063e1` (in the git log) was supposed to make all
dispatchers free functions, but only some were converted. Pick one
style and apply it consistently.

**Recommendation:** keep free functions for dispatchers
(`input_event`, `agent_event`, etc.) because they don't need
`&mut self` access to anything except the parameter — they could
operate on `&mut SessionState` if we wanted. Keep methods for things
that conceptually mutate a single subsystem (e.g.
`AppState::switch_model` is a method because it touches 5+ fields
that span multiple inner structs).

**`scroll_event` is 110 lines** — that's a single match with many arms.
Each arm is 1-3 lines, but the function is long. Split into
`update/scroll.rs` and move the page-size constant out of the function.

**Out of scope:**
- Rewriting the dispatch table to use a function-pointer array
- Changing `EventCategory` shape
- Adding new dispatch categories

**Verification:**
```bash
# mod.rs is small
wc -l crates/runie-core/src/update/mod.rs  # should be < 500

# No function in update/ is > 40 lines
for f in crates/runie-core/src/update/*.rs; do
  awk '/^pub fn |^fn |^pub\(crate\) fn /{...}' "$f" | awk '$1 > 40'
done

# Build + tests clean
cargo build --workspace
cargo test -p runie-core --lib
```
