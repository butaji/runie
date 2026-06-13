# Extract Login Flow from update/mod.rs Into a Focused File

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1
**Depends on**: resolve-merge-conflicts, fix-broken-references

## Description

The login flow is currently **entirely inlined in
`crates/runie-core/src/update/mod.rs`** (lines 232-595, ~360 lines). This
is the consequence of how the merge was resolved: the original
`update/login_flow.rs` (a sibling file with the same handlers) was
removed because it called a missing `build_login_stack` function (see
`fix-broken-references`). The richer, working version in `mod.rs` was
kept. The result: the dispatcher, the providers dialog, the login flow
panel-stack helpers, and the login flow state machine are all stuffed
into one 1901-line file.

The methods to extract (with current line numbers in
`crates/runie-core/src/update/mod.rs`):

| Method | Line | Purpose |
|---|---|---|
| `providers_event` | 232 | Routes `Event::ProvidersDialog`, `ProvidersSelectModel`, `ProvidersDisconnect`, `ProvidersAdd` |
| `open_providers_dialog` | 254 | Opens the providers PanelStack |
| `providers_select_model` | 268 | Switches active model from providers dialog |
| `providers_disconnect` | 276 | Removes a provider config |
| `login_flow_event` | 303 | Top-level login flow dispatcher (the `match event` with all 9 Login* events) |
| `login_flow_start` | 326 | Initializes `LoginFlowState::new()` |
| `login_flow_select_provider` | 331 | Sets the provider on the flow |
| `login_flow_submit_key` | 342 | Stores the API key + transitions to ModelSelect |
| `login_flow_validation_done` | 369 | Enriches the model list with fetched models |
| `login_flow_models_fetched` | 379 | Same (alias used by some callers) |
| `login_flow_validation_failed` | 389 | Surfaces a transient warning on validation failure |
| `login_flow_toggle_model` | 403 | Toggles a model in the multi-select |
| `login_flow_save` | 412 | Persists the provider config and clears the flow |
| `login_flow_cancel` | 450 | Cancels without saving |
| `pop_login_panel_or_close` | 461 | Android-like back-stack pop |
| `push_login_panel` | 495 | Push a panel onto the login stack |
| `replace_top_login_panel` | 512 | Rebuild the top panel from current state |
| `replace_top_login_panel_with` | 534 | Pop and push atomically (e.g. key input → model selector) |
| `take_or_create_login_stack` | 548 | Get the current login PanelStack or build a new root |
| `rebuild_login_dialog` | 556 | Open the rebuilt login dialog as the open dialog |

That's 21 methods / ~360 lines that all logically belong to the same
subsystem.

## Acceptance Criteria

- [ ] All 21 methods above are moved to a new `crates/runie-core/src/update/login_flow.rs` (re-creating the file that was archived)
- [ ] `update/mod.rs` shrinks by at least 300 lines (some `use` statements also move)
- [ ] The dispatcher in `update/mod.rs::update()` is updated to call the new methods (e.g. `self.login_flow_event(event)` becomes `login_flow::event(self, event)` or similar — pick a style and apply consistently)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test -p runie-core --lib tests::login_logout` passes (the 18+ test cases that cover `/providers`, login, logout, connect, disconnect)
- [ ] `cargo test -p runie-core --lib tests::slash::session` passes (slash commands that include LoginFlow* paths)
- [ ] `cargo test -p runie-core --lib login_flow::tests` passes (the data-model tests in `login_flow.rs` itself)
- [ ] The 12 scenario tests in `login_flow/tests/state.rs` (S1–S13 from the docstring in `login_flow.rs:9-23`) all pass
- [ ] `Event::LoginFlowValidate` is **either handled in `login_flow_event`** (it currently falls through to `_ => {}`) **or removed from the `Event` enum** (the current `event.rs:266` variant is dead in practice — see `clean-dead-modules` for the removal option)
- [ ] The new `update/login_flow.rs` re-export from `lib.rs` (if needed) is added — current `lib.rs:74` only re-exports `build_*` and `LoginFlowState` from the data model, not the handlers
- [ ] No function in the new file exceeds 40 lines (the build.rs lint)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo test -p runie-core --lib update::login_flow` passes (the new module's own tests, if any are added)
- [ ] `cargo test -p runie-core --lib login_flow::tests` passes (the data-model tests in `login_flow/tests/state.rs` — they test the state machine, not the event handlers, so they should pass without change)
- [ ] `cargo test -p runie-core --lib tests::login_logout` passes (18+ test cases that cover the integration)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib tests::slash` passes (slash commands that may open login flow as a side effect)
- [ ] `cargo test -p runie-core --lib tests::dialog_theme_switch` passes (theme switching can trigger re-render of dialogs)
- [ ] `cargo test -p runie-core --lib tests::stack_navigation` passes (Android-like back-stack semantics, directly exercises `pop_login_panel_or_close` and `push_login_panel`)

### Layer 4 — Smoke
- [ ] `tmux_login_logout_test.sh` runs end-to-end against a release build
- [ ] `./dev.sh` succeeds (the documented dev script)

## Notes

**Historical context (for future readers):**

There were once two files for the login flow: `login_flow.rs` (data
model + panel builders) and `update/login_flow.rs` (event handlers).
The handler file called `build_login_stack`, a function that did not
exist (it was renamed during a refactor). The merge conflict
resolution in commit `77a605c3` chose to keep the rich
`update/mod.rs` version and archive the broken handler file
(commit `0959861e`, now at
`crates/_archive/update-orphans/login_flow.rs`). This task
re-creates the handler file, fixing the build-time issue (call
`build_login_root` instead of `build_login_stack`) and moving the
methods out of `mod.rs`.

**The dispatcher in `update/mod.rs:42-220` is the call site.** The
relevant arms today:

```rust
if event.is_login() {  // line 56
    return self.login_flow_event(event);
}
if matches!(event, Event::ProvidersDialog | ...) {  // line 67
    return self.providers_event(event);
}
...
Event::LoginFlowStart | ... | Event::LoginFlowCancel  // line 214
    => self.login_flow_event(event),
```

After the move, these become `login_flow::event(self, event)` and
`login_flow::providers_event(self, event)`.

**The 21 methods form one logical subsystem.** They share private
state via `self.login_flow: Option<LoginFlowState>`, `self.open_dialog:
Option<DialogState>`, `self.dialog_back_stack: Vec<DialogState>`. No
cross-coupling with input/agent/view state. Safe to extract.

**`Event::LoginFlowValidate` is the loose end.** The event variant
exists in `event.rs:266` and is matched in two `if` arms in the
dispatcher (lines 57, 214) but the actual `login_flow_event` has
`_ => {}` at the end (line 322), so the event is dropped. Either:
- Add a real handler that calls `validate_api_key(meta.base_url, key)`
  and emits `LoginFlowValidationDone` / `LoginFlowValidationFailed`,
  then update `runie-term/src/main.rs:308-340` to use the new event
  path (it currently has the same logic inline)
- Or remove `Event::LoginFlowValidate` from the `Event` enum and
  document the removal in `CHANGELOG.md`

**Out of scope:**
- Splitting `login_flow.rs:200` `build_login_root` into a
  state-machine-driven `build_step(state, step)` (would require a real
  state machine)
- Refactoring the PanelStack manipulation in `pop_login_panel_or_close`
  etc. into a more idiomatic Rust API (the current `Vec<Panel>` with
  manual index management is correct but verbose)
- Wiring the archived `crates/_archive/update-orphans/login_flow.rs`
  back in (it's the loser; the in-`mod.rs` version is the winner)

**Verification:**
```bash
# mod.rs shrinks significantly
wc -l crates/runie-core/src/update/mod.rs  # should drop by ~300

# Build clean
cargo build --workspace

# All login-flow-related tests pass
cargo test -p runie-core --lib login_flow
cargo test -p runie-core --lib tests::login_logout
cargo test -p runie-core --lib tests::stack_navigation
```
