# Provider-config scan report

**Focus area:** provider building, API-key / config resolution, login/onboarding flow, runtime `ConfigState` vs saved `Config`, and turn-state reset on errors.  
**Workspace:** `/Users/admin/Code/GitHub/runie-dev` (branch `dev`).  
**Scanned:** `crates/runie-provider`, `crates/runie-agent`, `crates/runie-core` (commands/dialog/login/update), `crates/runie-tui`, `crates/runie-server`, `crates/runie-json`, `crates/runie-print`.

## Summary

The config-aware provider builders (`build_provider_with_warning_with_config`, `DynProvider::new_with_config`, `run_subagent_with_config`) are now used in the live production paths (TUI main loop, JSON/print CLIs, server, subagent effect). The main remaining risks are **stale config snapshots**, **divergence between runtime `ConfigState` and the saved `Config`**, and **custom `base_url` being ignored in the login flow**. A handful of legacy env-only builders are still public and could be reintroduced by future callers.

No TODO/FIXME/HACK comments were found in the provider-config area.

## Findings

### P1 — TUI agent loop holds a stale config snapshot

- **File:** `crates/runie-tui/src/main.rs`, lines 157–159 and 248
- **Description:** `provider_config` is loaded once at startup and moved into the long-running `agent_loop`. If the user completes `/login` and `login_config::save_provider_config` writes new credentials to `~/.runie/config.toml`, subsequent turns still build providers from the old snapshot, producing `MissingApiKey` even though the key is saved.
- **Suggested fix:** Reload the config inside `run_single_turn` (as `crates/runie-tui/src/effects/subagent.rs:18` already does) or pass an `Arc<RwLock<Config>>`/`watch` channel that the config watcher updates.

### P1 — Login save does not update the runtime config baseline

- **File:** `crates/runie-core/src/update/login_flow.rs`, lines 243–264 and `activate_first_selected_model` at 285–294
- **Description:** `login_flow_save` writes the provider under `[model_providers.<name>]` but never sets the top-level `provider`/`[models].default` fields or `state.config.config_provider`/`config_model`. `activate_first_selected_model` calls `state.switch_model`, which only updates `current_provider`/`current_model`. Later, `/new` (`crates/runie-core/src/commands/dsl/handlers/session/mod.rs:315–316`) resets the active model to the stale `config_*` values, which are empty in a fresh onboarding. The user is dumped back into the login flow after `/new`.
- **Suggested fix:** In `login_flow_save`, also write `provider = "..."` and `[models].default = "..."` to the config file, and update `state.config.config_provider`/`config_model` to match the newly active model.

### P2 — API-key validation ignores custom `base_url`

- **File:** `crates/runie-tui/src/effects/login.rs`, lines 13–20
- **Description:** The async validation effect calls `runie_provider::validate_api_key(meta.base_url, &key)`, where `meta.base_url` is the hard-coded registry default. It does not read the provider’s saved `base_url` from `~/.runie/config.toml`. Users with a proxy / local endpoint will fail validation even when the saved config has the correct `base_url`.
- **Suggested fix:** Load the current config and pass `ProviderConfigResolver::resolve_base_url(provider)` (or the saved value) to `validate_api_key`.

### P2 — Login save clobbers custom `base_url`

- **File:** `crates/runie-core/src/update/login_flow.rs`, lines 243–249
- **Description:** `login_flow_save` computes `base_url` from `provider_registry::find_provider(...).base_url` and writes that to `[model_providers.<name>]`. Any previously saved custom `base_url` is overwritten with the registry default.
- **Suggested fix:** Preserve the existing saved `base_url` when updating a provider entry, or allow the user to supply/edit it during onboarding.

### P2 — Runtime `ConfigState` and saved `Config` diverge on explicit model switches

- **File:** `crates/runie-core/src/update/system.rs`, lines 107–125
- **Description:** `switch_model` updates `current_provider`/`current_model` and records usage, but never updates `config_provider`/`config_model`. `/new` therefore reverts an explicit `/model` switch. The same applies to `cycle_model` (lines 156–177) and `set_provider` (lines 127–135).
- **Suggested fix:** Treat an explicit model switch as the new session default and update `config_provider`/`config_model` (or persist the choice to `config.toml`).

### P2 — `reload_all` does not pick up providers saved under `[model_providers]`

- **File:** `crates/runie-core/src/update/system.rs`, lines 385–406
- **Description:** `reload_all` only sets `config_provider`/`config_model` from `config.provider` / `config.default_model()`. Because onboarding writes to `[model_providers.<name>]`, not the top-level fields, `reload_all` leaves the runtime baseline empty after a fresh login.
- **Suggested fix:** Derive the default provider/model from the configured providers when the top-level fields are absent.

### P2 — Config watcher updates UI state but not the provider builder config

- **File:** `crates/runie-core/src/config_reload/watcher.rs`, lines 57–77
- **Description:** When `config.toml` changes externally, the watcher emits `SwitchModel`/`SwitchTheme`/`KeybindingsReloaded` events for the UI. The separate `Config` snapshot owned by the TUI agent loop is never refreshed, so a newly added provider key is not used until restart.
- **Suggested fix:** Share a single config source (e.g. `Arc<RwLock<Config>>` or a `watch::Receiver<Config>`) between the watcher and the agent loop.

### P2 — Legacy env-only builder functions are still public

- **Files:**
  - `crates/runie-provider/src/lib.rs:187–211` (`build_provider`, `build_provider_with_warning`)
  - `crates/runie-provider/src/lib.rs:239–246` (`switch_provider`)
  - `crates/runie-agent/src/lib.rs:39–50` (re-exports of env-only builders)
  - `crates/runie-agent/src/subagent.rs:33–55` (`run_subagent` wrapper that uses `Config::default()`)
- **Description:** These functions only look at environment variables. They are not used by current production binaries, but they remain in the public API and could be reintroduced by future callers, re-creating the exact bug the recent migration fixed.
- **Suggested fix:** Mark them `#[deprecated]` with a note pointing to the `_with_config` variants, or remove/restrict them to test code.

### P3 — `DynProvider::new_checked` duplicates `new`

- **File:** `crates/runie-provider/src/lib.rs:63–65`
- **Description:** `new_checked` has the same signature and body as `new` and is not used anywhere.
- **Suggested fix:** Remove it or give it a distinct purpose (e.g. return the key on error).

## Error-handling / turn-state observations

- `AgentEvent::Error` correctly resets turn/streaming state via `AppState::add_error` (`crates/runie-core/src/update/agent/core.rs:331–362`). Tests cover this in `crates/runie-core/src/tests/agent_error.rs`.
- When a provider build fails in the TUI, `run_single_turn` publishes `AgentEvent::Error` and returns early without `Done`; `add_error` already clears the state, so no stuck "Working…" status is left.
- In the subagent path, an `AgentEvent::Error` sets `SubagentState.error`; `finalize_subagent_result` checks the error before the `done` flag, so the subagent returns the actual error rather than "subagent did not finish".
- No additional error-path state leaks were identified in the provider-config area.

## Test gaps

- **Runtime config refresh:** No test simulates "save provider during onboarding → send a message" in the TUI agent loop. The existing `provider_config_e2e.rs` only verifies that `build_provider_with_warning_with_config` works from a freshly loaded file.
- **`/new` after onboarding:** No test asserts that `/new` preserves the provider selected during login.
- **Custom `base_url` round-trip:** No test covers saving, validating, and re-building a provider with a non-default `base_url`.
- **`config_provider`/`config_model` updates:** No test verifies these fields are updated after `/model`, login save, or `reload_all`.
- **Env-only builders:** No lint or test prevents future production callers from using `build_provider_with_warning` / `DynProvider::new` / `run_subagent`.

## Linter-adjacent risks

The build script at `crates/runie-core/build.rs` enforces a 500-line file cap. Several files in the provider-config neighborhood are already at or near the limit:

- `crates/runie-agent/src/turn.rs` — 500 lines
- `crates/runie-core/src/update/dialog/panel.rs` — 500 lines
- `crates/runie-core/src/dialog/panel.rs` — 494 lines
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — 470 lines
- `crates/runie-core/src/update/system.rs` — 469 lines
- `crates/runie-core/src/update/login_flow.rs` — 460 lines
- `crates/runie-core/src/model_catalog.rs` — 460 lines

Function-length risks (40-line production cap) in the same area:

- `crates/runie-agent/src/turn.rs:194–233` — `run_agent_iteration` is already at the 40-line boundary.
- `crates/runie-agent/src/turn.rs:299–337` — `execute_tools` is close to the boundary.
- `crates/runie-core/src/update/login_flow.rs:228–264` — `login_flow_save` is ~36 lines and will grow once config-baseline updates are added.

Adding the fixes above without extracting helpers is likely to trigger build failures; factor new logic into small functions.
