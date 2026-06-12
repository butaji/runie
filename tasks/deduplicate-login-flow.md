# Deduplicate Login Flow Handlers Across Three Files

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1
**Depends on**: resolve-merge-conflicts, fix-broken-references

## Description

The login flow has THREE sets of handlers:

1. **`crates/runie-core/src/login_flow.rs`** — the data model
   (`LoginFlowState`, `LoginStep`, `with_provider`, `with_key_and_defaults`,
   `with_fetched_models`, `toggle_model`) and the panel builders
   (`build_provider_picker`, `build_key_input`, `build_model_selector`,
   `build_done_panel`, `build_login_root`).

2. **`crates/runie-core/src/update/login_flow.rs`** — 121 lines of
   `impl AppState` methods: `login_flow_start`,
   `login_flow_select_provider`, `login_flow_submit_key`,
   `provider_defaults`, `login_flow_validation_done`,
   `login_flow_models_fetched`, `login_flow_validation_failed`,
   `login_flow_toggle_model`, `login_flow_save`, `login_flow_cancel`,
   `rebuild_login_dialog`. This file is called from
   `update/mod.rs::update()` via `login_flow::update(state, event)`.

3. **`crates/runie-core/src/update/mod.rs`** — the merge-conflict
   survivors include another set of login-flow methods on
   `AppState`: `login_flow_event`, `login_flow_select_provider`,
   `login_flow_submit_key`, `login_flow_validation_done`,
   `login_flow_models_fetched`, `login_flow_validation_failed`,
   `login_flow_toggle_model`, `login_flow_save`, `login_flow_cancel`,
   plus panel-stack helpers `pop_login_panel_or_close`,
   `push_login_panel`, `replace_top_login_panel`,
   `replace_top_login_panel_with`, `take_or_create_login_stack`,
   `rebuild_login_dialog`, `providers_event`, `open_providers_dialog`,
   `providers_select_model`, `providers_disconnect`.

The `update/login_flow.rs` versions are simpler (no panel-stack
manipulation) and the `update/mod.rs` versions are richer (with
back-stack semantics). Only one set is actually invoked from the
dispatcher; the other is dead code that the merge conflict
preserved.

## Acceptance Criteria

- [ ] All `login_flow_*` and `rebuild_login_dialog` methods exist in exactly ONE place: `crates/runie-core/src/update/login_flow.rs`
- [ ] The `provider_defaults`, `pop_login_panel_or_close`, `push_login_panel`, `replace_top_login_panel`, `replace_top_login_panel_with`, `take_or_create_login_stack` helpers exist in exactly ONE place: `crates/runie-core/src/update/login_flow.rs`
- [ ] The `providers_event`, `open_providers_dialog`, `providers_select_model`, `providers_disconnect` methods also move to `update/login_flow.rs` (they're conceptually part of the same flow, just at a different step)
- [ ] `update/mod.rs` has zero `AppState` methods related to login flow
- [ ] `update/mod.rs::update()` still calls `login_flow::update(self, event)` for the `is_login` branch (no behavior change)
- [ ] `update/mod.rs` shrinks by at least 200 lines after the move
- [ ] All 13 `Event::LoginFlow*` variants are handled in `update/login_flow.rs::update()` with no `_ => {}` fallthrough (the current `_ => {}` in `update/login_flow.rs:25` is suspicious — the `LoginFlowValidate` event is unhandled)
- [ ] The `LoginFlowValidate` event from `event.rs:255` is either handled or marked `#[allow(dead_code)]` with a TODO

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo build --workspace --tests` succeeds
- [ ] `cargo test -p runie-core --lib update::login_flow` passes (the consolidated module)
- [ ] `cargo test -p runie-core --lib login_flow::tests` passes (the data model tests)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib tests::login_logout` passes (covers `/providers`, login, logout, connect, disconnect — 668 lines, 18+ test cases)
- [ ] `cargo test -p runie-core --lib tests::slash::session` passes (slash commands that include `LoginFlow*` paths)
- [ ] The S1–S13 scenarios from `login_flow/tests/state.rs` (provider picker → key input → model select → save) all pass

### Layer 4 — Smoke
- [ ] `tmux_login_logout_test.sh` passes end-to-end

## Notes

**Which set to keep:**

The `update/mod.rs` versions are richer because they implement the
Android-like back-stack semantics (push the current dialog onto the
back stack when opening a sub-dialog). The `update/login_flow.rs`
versions are simpler and do not manipulate the panel stack — they
rebuild the entire dialog from scratch on every state change.

**Recommendation:** keep the `update/mod.rs` version (with back-stack
semantics) and move it to `update/login_flow.rs`. The current
`update/login_flow.rs` is the dead one. The dispatcher in
`update/mod.rs::update()` already routes to `login_flow::update(state, event)`,
so moving the methods preserves the call site.

**Conflict markers in `update/mod.rs` lines 270-440** contain a
**partial** copy of the login-flow handlers. After resolving the
conflict (per `resolve-merge-conflicts`), this task is the followup
to move the *winning* version to `update/login_flow.rs` and
delete the *losing* version (and the partial duplicate) from
`update/mod.rs`.

**`LoginFlowValidate` event is unhandled** — it appears in `event.rs:255`
but `update/login_flow.rs:25` has `_ => {}` so it gets dropped. The
event has a `provider` and `key` payload. Either:
- Add a handler that triggers `validate_api_key(base_url, key)` and emits `LoginFlowValidationDone` or `LoginFlowValidationFailed` — this is the right behavior per the non-blocking design in `login_flow.rs:11-19`
- Or delete the event variant from `Event`

**Out of scope:**
- Splitting `login_flow.rs:200` `build_login_root` into a `build_step(state, step)` that returns the right panel per step (would require a real state machine; the current `build_*_panel` functions are fine)
- Refactoring `LoginFlowState` to use a state-machine library
- Adding tests for `LoginFlowValidate` (separate task)

**Verification:**
```bash
# Login-flow methods exist in exactly one place
git grep -nE 'fn (login_flow_start|login_flow_select_provider|login_flow_submit_key|login_flow_validation_done|login_flow_models_fetched|login_flow_validation_failed|login_flow_toggle_model|login_flow_save|login_flow_cancel|rebuild_login_dialog|provider_defaults|pop_login_panel_or_close|push_login_panel|replace_top_login_panel|take_or_create_login_stack|providers_event|open_providers_dialog|providers_select_model|providers_disconnect)\b' -- 'crates/runie-core/src/'

# Build + tests clean
cargo build --workspace
cargo test -p runie-core --lib
```
