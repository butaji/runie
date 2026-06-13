# Split crates/runie-core/src/update/mod.rs Into Focused Files

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1
**Depends on**: extract-login-flow

## Description

`crates/runie-core/src/update/mod.rs` is 1901 lines — 901 over the
project's relaxed 1000-line cap (raised from 500 in commit `402943c5`).
It contains:

- The main `AppState::update` dispatcher (lines 41-220, ~180 lines)
- 11 method dispatchers on `AppState` (all now `&mut self` methods, not
  the free functions I described in the previous version of this task):
  - `input_event` (in `update/input.rs:482`, not mod.rs)
  - `agent_event` (in `update/agent.rs:302`, not mod.rs)
  - `scroll_event` (mod.rs:573, **110 lines**)
  - `control_event` (in `update/control.rs:50`)
  - `model_config_event` (mod.rs:734, ~25 lines)
  - `dialog_toggle_event` (mod.rs:752, ~40 lines)
  - `login_flow_event` (mod.rs:303, ~25 lines)
  - `providers_event` (mod.rs:232, ~25 lines)
  - `update_dialog` (mod.rs:1157, ~70 lines)
  - `update_panel_stack` (mod.rs:1224, ~60 lines)
  - `update_form_panel` (mod.rs:1283, ~40 lines)
- 21 login flow + providers dialog methods (mod.rs:232-595, ~360 lines) — **already
  extracted in concept by `extract-login-flow`**, see that task
- 20+ dialog/form/command-result methods (mod.rs:783-1830, ~1050 lines)
- The `FormAction` enum (mod.rs:11-26)
- The `partition_model_items` helper (mod.rs:1893+)

The merge resolution in commit `77a605c3` rewrote the dispatcher as a
single large `match event` (rather than the previous
`EventCategory`-based routing). This is correct but the file is now
even more bloated.

## Acceptance Criteria

- [ ] `crates/runie-core/src/update/mod.rs` is ≤ 500 lines (the
  original, strict cap)
- [ ] The main `AppState::update` dispatcher stays in `mod.rs` but is
  ≤ 200 lines (the dispatcher IS long because it has 12+ match arms,
  but each arm is small)
- [ ] The 11 method dispatchers move to focused files. The pattern
  to follow: `update/scroll.rs` (for `scroll_event`), `update/control.rs`
  (for `control_event`, may need a rename to `control_event.rs` or
  be moved into existing files), etc.
- [ ] The dialog system methods (`update_dialog`, `update_panel_stack`,
  `update_form_panel`, `form_panel_action`, `form_panel_edit_char`,
  `form_build_submit`, `form_dialog_event`, `apply_form_action`,
  `try_activate_panel`, `handle_panel_action`, `panel_toggle_item`,
  `panel_cycle_item`, `apply_panel_setting`, `process_command_result`,
  `push_dialog_to_back_stack`, `open_at_file_picker`, `insert_at_ref`)
  move to a new `update/dialog.rs` (consolidating the existing
  `update/dialog_actions.rs`, `update/dialog_open.rs`,
  `update/dialog_update.rs`)
- [ ] The `FormAction` enum moves to `update/form.rs`
- [ ] The `partition_model_items` helper moves to
  `update/model_selector.rs` (which already exists at 106 lines)
- [ ] Login flow methods (21 of them) are already in
  `update/login_flow.rs` per `extract-login-flow`
- [ ] Open-command methods (`open_command_palette`, `open_model_selector`,
  `open_settings_dialog`, `open_scoped_models_dialog`,
  `open_session_tree_dialog`, `toggle_dialog`) move to
  `update/dialog.rs` (they're open-the-dialog glue, not state changes)
- [ ] `update/scoped_models.rs` keeps all scoped-models methods; the
  duplicates in `mod.rs` are removed
- [ ] No function in the split files exceeds 40 lines
- [ ] No file in `crates/runie-core/src/update/` exceeds 500 lines
- [ ] `build.rs`'s `ALLOWED_FILES_OVER` is updated to remove
  `crates/runie-core/src/update/mod.rs` (it's no longer over the cap)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds after the split
- [ ] `cargo build --workspace --tests` succeeds
- [ ] `cargo test -p runie-core --lib update::` passes (the
  dispatcher still routes events)
- [ ] `cargo test -p runie-core --lib update::scroll` passes (if
  scroll tests are added)
- [ ] `cargo test -p runie-core --lib update::dialog` passes
- [ ] `cargo test -p runie-core --lib update::form` passes
- [ ] `cargo test -p runie-core --lib update::login_flow` passes
  (per `extract-login-flow`)

### Layer 2 — Event Handling
- [ ] All 595+ existing `#[test]` annotations in
  `crates/runie-core/src/tests/` still pass
- [ ] `cargo test -p runie-core --lib tests::dialog_theme_switch`
  passes
- [ ] `cargo test -p runie-core --lib tests::form_dialog` passes
- [ ] `cargo test -p runie-core --lib tests::model_selector` passes

## Notes

**Current file sizes** in `update/` (after `extract-login-flow`
moves 21 methods out):

```
1901 crates/runie-core/src/update/mod.rs     ← split target
 482 crates/runie-core/src/update/input.rs
 302 crates/runie-core/src/update/agent.rs
 250 crates/runie-core/src/update/edit.rs
 240 crates/runie-core/src/update/line_nav.rs
 218 crates/runie-core/src/update/dialog_open.rs
 200 crates/runie-core/src/update/dialog_actions.rs
 164 crates/runie-core/src/update/queue.rs
 152 crates/runie-core/src/update/tab_complete.rs
 133 crates/runie-core/src/update/settings_dialog.rs
 128 crates/runie-core/src/update/dialog_update.rs
 118 crates/runie-core/src/update/palette.rs
 106 crates/runie-core/src/update/model_selector.rs
  93 crates/runie-core/src/update/system_actions.rs
  90 crates/runie-core/src/update/dialog.rs          ← already exists
  ... (others under 100 lines)
```

Even after `extract-login-flow` removes ~360 lines, `mod.rs` is
~1540 lines. This task is the next step.

**Proposed file layout after the split:**

```
crates/runie-core/src/update/
├── mod.rs              (≤ 500 lines: dispatcher only)
├── at_refs.rs          (existing, 18 lines)
├── agent.rs            (existing, 302 lines, +agent_event free fn)
├── bash.rs             (existing, 93 lines)
├── control.rs          (existing, 50 lines, +control_event)
├── dialog.rs           (existing 90 lines, GROW to ~600: all dialog glue)
├── dialog_actions.rs   (existing, FOLD into dialog.rs)
├── dialog_open.rs      (existing, FOLD into dialog.rs)
├── dialog_update.rs    (existing, FOLD into dialog.rs)
├── edit.rs             (existing, 250 lines)
├── edit_approval.rs    (existing, 34 lines)
├── form.rs             (NEW, ~150 lines: FormAction + form_panel_*)
├── input.rs            (existing, 482 lines, +input_event)
├── input_scroll.rs     (existing, 49 lines)
├── input_text.rs       (existing, 62 lines)
├── line_nav.rs         (existing, 240 lines)
├── login_flow.rs       (from extract-login-flow, ~360 lines)
├── model_selector.rs   (existing, 106 lines, +partition_model_items)
├── palette.rs          (existing, 118 lines)
├── path_complete.rs    (existing, 75 lines)
├── queue.rs            (existing, 164 lines)
├── scroll.rs           (NEW, ~80 lines: scroll_event only)
├── scoped_models.rs    (existing, 44 lines)
├── session.rs          (existing, 67 lines)
├── settings_dialog.rs  (existing, 133 lines)
├── system_actions.rs   (existing, 93 lines, +system_event)
└── tab_complete.rs     (existing, 152 lines)
```

**`scroll_event` is 110 lines** — the longest function in `mod.rs`.
Splitting it into per-key cases (e.g. `scroll_up`, `scroll_down`,
`page_up`, `page_down`) would shrink each case to ~10 lines.

**`update_dialog` is ~70 lines** — a single match with 12+ arms
across 3 dispatch paths (form panel, list panel, dialog-back). This
is the dialog system in one function. It should be split into:
- `route_dialog_event(state, event, stack)` — the top-level switch
- `route_form_event(state, event, panel)` — form-specific
- `route_list_event(state, event, panel)` — list-specific

**The dispatcher is long but mechanical.** Each arm is
`Event::Foo | Event::Bar => self.method(event),`. Could be table-
driven:

```rust
const INPUT_EVENTS: &[EventCategory] = &[EventCategory::Input, ...];
match event.category() {
    EventCategory::Input if INPUT_EVENTS.contains(&event.category()) => input_event(self, event),
    ...
}
```

But the current style (explicit match arms) is more readable and
catches misspellings at compile time. Don't refactor the dispatcher
itself; just move the per-category handlers out.

**Out of scope:**
- Rewriting the dispatch table to use a function-pointer array
  (would lose the exhaustiveness check)
- Changing `EventCategory` shape
- Adding new dispatch categories
- Consolidating `update/dialog.rs`, `update/dialog_actions.rs`,
  `update/dialog_open.rs`, `update/dialog_update.rs` into a single
  `update/dialog.rs` (this IS in scope for this task)

**Verification:**
```bash
# mod.rs is small
wc -l crates/runie-core/src/update/mod.rs  # should be < 500

# No function in update/ is > 40 lines (or in the new allow-list)
for f in crates/runie-core/src/update/*.rs; do
  awk '...' "$f"
done

# Build + tests clean
cargo build --workspace
cargo test -p runie-core --lib
```
