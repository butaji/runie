# Scan Report: slash-commands-forms

**Workspace:** `/Users/admin/Code/GitHub/runie-dev`  
**Focus:** slash commands, form dialogs, provider building, agent turn state reset  
**Date:** 2026-06-18  

## Summary

The slash-commands-forms area is mostly healthy after the recent provider-config migration, but there are **three high-impact regressions/gaps** that can break real user flows:

1. The TUI `agent_loop` captures the saved `Config` once at startup. After the user completes `/login` or changes `~/.runie/config.toml`, the agent still builds providers from the stale snapshot and can fail with `MissingApiKey` even though the key was just saved.
2. `AgentEvent::Error` resets the obvious turn flags but leaves several per-turn counters/buffers intact (`intermediate_step_count`, `thought_seq`, `streaming_buffer`, `last_assistant_index`, speed tracking). This can corrupt tool IDs on the next turn or append stale streamed content to the wrong assistant message.
3. `ConfigState` is initialized with empty provider/model in production, so users with saved providers are forced back into the login flow on every startup instead of having their last provider restored.

Additional medium-severity issues include a `/reload` that updates `config_provider`/`config_model` but not `current_provider`/`current_model`, a skill tool-abort path that panics, and a form-builder that drops hidden field values when slash args are pre-filled.

---

## Findings

### P1 — High

#### 1. `agent_loop` uses a startup snapshot of `Config` and never refreshes it
- **File:** `crates/runie-tui/src/main.rs`
- **Lines:** 155–159, 230–236
- **Severity:** P1
- **Description:** `spawn_background_tasks` loads `Config` once and moves it into the long-lived `agent_loop`. Every subsequent turn calls `build_provider_with_warning_with_config(..., &config)` with that captured snapshot. When the user adds a provider through the login flow, runs `/reload`, or edits `config.toml`, the on-disk config changes but the agent loop keeps using the old in-memory struct. The next message can fail with a misleading `Missing API key` error even though the key is saved. The subagent effect (`crates/runie-tui/src/effects/subagent.rs:18`) already avoids this by loading config inside the spawned task.
- **Suggested fix:** Remove the captured `config` from `agent_loop` and load `runie_core::config::Config` fresh inside `run_single_turn` (or share an `Arc<tokio::sync::RwLock<Config>>` that the config watcher updates).

#### 2. `AgentEvent::Error` does not fully reset per-turn state
- **File:** `crates/runie-core/src/update/agent/core.rs`
- **Lines:** 331–362 (`add_error`)
- **Severity:** P1
- **Description:** `add_error` resets `turn_active`, `streaming`, `current_request_id`, `inflight`, timers, `current_tool_name`, and `current_action`, but it does **not** reset `intermediate_step_count`, `thought_seq`, `streaming_buffer`, `last_assistant_index`, `turn_tokens_out`, `speed_tps`, `last_speed_update`, or `tokens_at_last_speed`. On the next turn, tool IDs continue counting from the failed turn (`tool.{id}.{N}`), and if a stray `Done` follows the error, `finish_turn` may flush the stale `streaming_buffer` into `last_assistant_index`, corrupting a previous assistant message.
- **Suggested fix:** Extract a single `reset_turn_state` helper that `clear_turn_state` and `add_error` both call, ensuring every per-turn field is zeroed/nulled. Add Layer-1 tests verifying `intermediate_step_count`, `thought_seq`, `streaming_buffer`, and `last_assistant_index` are cleared after `AgentEvent::Error`.

#### 3. Saved provider/model are not restored into `ConfigState` at startup
- **Files:** `crates/runie-core/src/state/session.rs:65–102`, `crates/runie-tui/src/app_init.rs:106–115`, `crates/runie-tui/src/main.rs:117–120`
- **Severity:** P1
- **Description:** `ConfigState::default()` leaves `current_provider`, `current_model`, `config_provider`, and `config_model` empty in production. `run_init_hooks` then opens the login flow whenever `current_provider.is_empty()`. A user who already saved a provider in a previous session is therefore forced to re-run the onboarding flow instead of starting with their saved model active. The config watcher can switch models *after* a file change, but it cannot restore an initial active model because the startup state never reads it.
- **Suggested fix:** In `app_init::init_ui_config` or a new `app_init::init_active_model`, load `Config::load(...)` and, when a saved provider/model exists and no mock is enabled, call `state.switch_model(provider, model)` (or at least set `config_provider`/`config_model`). Add a Layer-2 test asserting that startup with a saved provider does not open the login flow.

---

### P2 — Medium

#### 4. `/reload` updates saved-config defaults but leaves the active model unchanged
- **File:** `crates/runie-core/src/update/system.rs:385–406`
- **Lines:** 386–392
- **Severity:** P2
- **Description:** `reload_all` reads the saved `Config` and updates `config_provider`/`config_model`, but it never updates `current_provider`/`current_model`. If the user changes the default model in `config.toml` and runs `/reload`, the UI keeps using the old model. This diverges the runtime `ConfigState` from the saved `Config`.
- **Suggested fix:** When `config.provider` or `config.default_model()` differ from the current active model, call `state.switch_model(...)` unless the user has explicitly overridden the model in this session. Add a Layer-2 test for `/reload` switching the active model.

#### 5. `/reload` command handler reloads config incompletely
- **File:** `crates/runie-core/src/commands/dsl/handlers/system.rs:172–176`
- **Severity:** P2
- **Description:** `handle_reload` loads `Config` but only refreshes `keybindings`; it then emits `ReloadAll`, which updates provider defaults, theme, skills, prompts, and `vim_mode`. It does **not** re-initialize `scoped_models`, `truncation`, `telemetry`, or re-apply trust/read-only state, so a `/reload` after changing those sections has no effect. This is inconsistent with `app_init`, which loads each section separately.
- **Suggested fix:** Consolidate all reloadable sections in `AppState::reload_all` (or a dedicated `app_init::reload_all` helper) and have `handle_reload` call that single function. Add Layer-2 tests for truncation/scoped-model/telemetry reload.

#### 6. `build_form_stack_from_template` drops hidden form values when args are pre-filled
- **File:** `crates/runie-core/src/commands/dsl/builder.rs`
- **Lines:** 160–187
- **Severity:** P2
- **Description:** When a slash command like `/save mysession` pre-fills form fields, the builder reconstructs a new `Panel` and only copies `FormField`/`FormSubmit` items. Values added with `Panel::form_hidden` live in `built.form_values` and are lost in the rebuilt panel. The only current consumer of hidden values is the login key form (`login_flow/panels.rs:36`), which does not use slash-arg prefill, but the API is silently broken for any future form that combines hidden values with args.
- **Suggested fix:** After creating the new panel, copy `built.form_values` into it (`panel.form_values = built.form_values.clone();`) before applying pre-filled args. Add a Layer-1 test in `builder.rs` verifying a hidden value survives arg prefill.

#### 7. Skill tool-abort panics instead of returning an error
- **File:** `crates/runie-agent/src/turn.rs`
- **Lines:** 415–420 (`check_tool_call_before_hook`)
- **Severity:** P2
- **Description:** If a skill hook returns `ToolCallResult::Abort(reason)`, the agent panics with `"Tool abort not implemented in this path"`. This crashes the whole process rather than surfacing a controlled error to the user.
- **Suggested fix:** Return `ToolOutput` with `ToolStatus::Blocked`/`Error` and the abort reason, or emit `AgentEvent::Error` and end the turn cleanly. Add a Layer-1 test for skill tool abort.

#### 8. `switch_provider` still uses the env-only builder
- **File:** `crates/runie-provider/src/lib.rs`
- **Lines:** 238–246
- **Severity:** P2
- **Description:** `DynProvider::switch_provider` calls `build_dyn_provider(key, model, None)`, so it ignores API keys saved in `~/.runie/config.toml`. The function is currently unused in the workspace, but it is public API; any future caller will break for users who rely on saved config.
- **Suggested fix:** Either remove `switch_provider` (it has no callers) or change its signature to accept `&Config` and pass it to `build_dyn_provider`.

#### 9. `handle_form_dialog` is redundant and mishandles `FormAction::Back`
- **File:** `crates/runie-core/src/update/dialog/form_handler.rs:10–29`
- **Severity:** P2
- **Description:** When a dialog is open, form events are routed through `update_dialog` → `update_form_panel`, which correctly handles `Back` (pop/close). The separate `handle_form_dialog` path is only reachable when no dialog is open, where it immediately returns. If it were ever reached with an open dialog, its `apply_form_action(FormAction::Back)` would be a no-op, leaving `CommandFormClose` stuck. Keeping both paths invites future routing bugs.
- **Suggested fix:** Remove `handle_form_dialog` and the `DialogEvent::CommandForm*` branch in `dispatch_dialog_event`; ensure all form events flow through `update_dialog`/`update_form_panel`. If retention is required, make `handle_form_dialog` delegate to `update_dialog`.

#### 10. Test gaps in error-state and provider-config coverage
- **Files:** `crates/runie-core/src/tests/agent_error.rs`, `crates/runie-core/src/tests/reload.rs`, `crates/runie-tui/src/tests/provider_config_e2e.rs`
- **Severity:** P2
- **Description:**
  - `agent_error.rs` verifies timers/inflight/turn flags but not `intermediate_step_count`, `thought_seq`, `streaming_buffer`, or `last_assistant_index`.
  - `reload.rs` only asserts the `ReloadAll` event and keybindings length; it does not verify provider/model/theme/truncation/scoped-models reload.
  - `provider_config_e2e.rs` tests building a provider from a freshly loaded config, not the real TUI path where `agent_loop` holds a captured config.
- **Suggested fix:** Add the missing Layer-1/Layer-2 tests described above.

---

### P3 — Low / Linter-adjacent

#### 11. `turn.rs` is at the 500-line file limit and contains several 39–40 line functions
- **File:** `crates/runie-agent/src/turn.rs`
- **Lines:** 34–73, 101–139, 194–233, 235–271, 299–337
- **Severity:** P3
- **Description:** The file is exactly 500 lines and has five production functions at or near the 40-line function limit. Any small addition (e.g., the recommended error-state reset) will trip the build linter.
- **Suggested fix:** Refactor `run_agent_turn_with_skills`, `emit_turn_end`, `run_agent_iteration`, `stream_response`, and `execute_tools` by extracting smaller helpers before adding new logic.

#### 12. `panel.rs` and `update/dialog/panel.rs` are close to the file limit
- **Files:** `crates/runie-core/src/dialog/panel.rs` (494 lines), `crates/runie-core/src/update/dialog/panel.rs` (500 lines)
- **Severity:** P3
- **Description:** Both files are near/at the 500-line cap. `update/dialog/panel.rs` already hits the limit.
- **Suggested fix:** Move the test modules to `tests/` files or split panel update logic (filter/navigation/form) into submodules.

#### 13. `session/mod.rs` and `agents.rs` are approaching the file limit
- **Files:** `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` (470 lines), `crates/runie-core/src/commands/dsl/handlers/agents.rs` (414 lines)
- **Severity:** P3
- **Description:** Approaching the 500-line cap; adding new session form commands or agent-manager logic will soon require splitting.
- **Suggested fix:** Split `session/mod.rs` into `io.rs` and `run.rs` as the module comments already suggest; extract helper modules from `agents.rs`.

#### 14. `/new` does not close an open dialog
- **File:** `crates/runie-core/src/commands/dsl/handlers/session/mod.rs:309–324`
- **Severity:** P3
- **Description:** `handle_new` clears session/input/agent/message state but leaves any open dialog or back stack intact. A user who runs `/new` while a form is open will see the new session behind the still-open dialog.
- **Suggested fix:** Clear `state.open_dialog` and `state.dialog_back_stack` in `handle_new` (or decide this is intentional and document it).

---

## Recommended Priority Order

1. Fix `agent_loop` config staleness (P1 #1).
2. Fully reset per-turn state in `add_error` (P1 #2).
3. Restore saved provider/model on startup (P1 #3).
4. Make `/reload` consistent and complete (P2 #4, #5).
5. Fix form hidden-value loss and skill abort panic (P2 #6, #7).
6. Remove/update dead `switch_provider` API and redundant `handle_form_dialog` (P2 #8, #9).
7. Backfill tests for the above (P2 #10).
8. Refactor large files/functions to stay under linter limits (P3 #11–#13).
