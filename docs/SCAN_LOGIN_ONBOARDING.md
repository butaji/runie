# Scan Report: login-onboarding

Focus area: login flow, provider onboarding, provider config persistence, and the
runtime/saved-config boundary.

## Summary

The onboarding flow is well covered by Layer 2/3 tests, but there are several
production-path issues around config lifecycle: the TUI agent loop loads a
single config snapshot at startup, the startup gate decides whether to force
login without reading saved providers, and validation responses are not matched
to the in-flight provider/key. Together these can strand a user in the login
dialog after they have already onboarded, or cause the first message after
onboarding to fail with a missing API key.

No TODO/FIXME/HACK comments were found in the focus files. The concrete issues
below are actionable and mostly require small, targeted fixes.

---

## Findings

### P1 — TUI agent loop uses a stale config snapshot

- **File:** `crates/runie-tui/src/main.rs`
- **Lines:** 155–159, 240–257
- **Severity:** P1
- **Description:** `spawn_background_tasks` loads `provider_config` once and
  passes it to `agent_loop`. `run_single_turn` builds the provider with that
  snapshot. If the user completes the login flow after startup, the saved
  `model_providers` entry is written to disk but the running agent loop still
  holds the pre-onboarding config, so the first chat message after onboarding
  fails to build the provider.
- **Suggested fix:** Reload the config inside `run_single_turn` (or at the top
  of each `agent_loop` iteration) before calling
  `build_provider_with_warning_with_config`. Alternatively, pass a callback
  that loads the current config on demand.

### P1 — Startup login gate ignores saved providers

- **File:** `crates/runie-tui/src/main.rs` and `crates/runie-tui/src/app_init.rs`
- **Lines:** `main.rs` 117–120; `app_init.rs` 106–109 (and related init fns)
- **Severity:** P1
- **Description:** `run_init_hooks` auto-opens the login flow when
  `state.config.current_provider.is_empty()`. `AppState::default()` never loads
  `~/.runie/config.toml`, and none of the `app_init::*` helpers initialize the
  runtime provider/model from saved config. If the user already saved a
  provider in a previous session, the app still forces them into the
  non-closable login dialog on restart.
- **Suggested fix:** Load `config_reload::Config` at startup and, when no
  runtime model is active but `model_providers` (or top-level `provider`/`model`)
  exists, activate the first configured provider/model before deciding whether
  to open the login flow.

### P1 — Stale validation responses can advance the wrong flow

- **File:** `crates/runie-core/src/update/login_flow.rs`
- **Lines:** 176–187 (`login_flow_validation_done`), 202–217
  (`login_flow_validation_failed`)
- **Severity:** P1
- **Description:** `ModelsFetched`/`ValidationFailed` carry `provider` and `key`
  fields, but the handlers only check `flow.step == Validating`. If the user
  submits a key for provider A, quickly goes back, and submits a key for
  provider B, a late response for A will be applied to B’s flow, putting B into
  `ModelSelect` with A’s model list.
- **Suggested fix:** Ignore the event unless `event.provider == flow.provider`
  and `event.key == flow.key` (or at least `event.provider == flow.provider`).

---

### P2 — Onboarding save does not update config defaults or `/new` state

- **File:** `crates/runie-core/src/update/login_flow.rs` and
  `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- **Lines:** `login_flow.rs` 228–264 (`login_flow_save`); `session/mod.rs`
  315–316 (`handle_new`)
- **Severity:** P2
- **Description:** `login_flow_save` writes the provider into
  `[model_providers]` but does not write top-level `provider`/`model`, nor does
  it update `state.config.config_provider`/`config_model`. Later, `/new` resets
  the active model to `config_provider`/`config_model`, which are still empty,
  effectively disconnecting the user after onboarding.
- **Suggested fix:** When the first model is activated during onboarding, also
  set `state.config.config_provider`/`config_model` and persist top-level
  `provider`/`model` (or `[models].default`) to the config file.

### P2 — Empty env vars shadow saved config keys

- **File:** `crates/runie-provider/src/config.rs` and
  `crates/runie-provider/src/lib.rs`
- **Lines:** `config.rs` 59–68 (`resolve_api_key`); `lib.rs` 124–139
  (`resolve_credentials`)
- **Severity:** P2
- **Description:** `ProviderConfigResolver` and `resolve_credentials` treat an
  empty env var as a valid value and return it before falling back to the saved
  config key. A user who has `MINIMAX_API_KEY=` will see “Missing API key”
  even after a successful onboarding save.
- **Suggested fix:** Skip empty strings when resolving; env/dotenv should only
  win when they contain a non-empty value.

### P2 — Malformed config silently reverts to defaults

- **File:** `crates/runie-core/src/config.rs`
- **Lines:** 195–216 (`Config::load`)
- **Severity:** P2
- **Description:** If `~/.runie/config.toml` exists but is malformed,
  `Config::load` silently returns `Config::default()`. Combined with the
  startup login gate, this can force the user into the onboarding flow and
  appear to erase existing providers.
- **Suggested fix:** Surface a transient error or warning when a config file
  exists but cannot be parsed, so the user knows data may be ignored.

### P2 — `ValidationDone` is dead/duplicate event plumbing

- **File:** `crates/runie-core/src/event/variants.rs` and
  `crates/runie-core/src/update/login_flow.rs`
- **Lines:** `variants.rs` 361–365; `login_flow.rs` 100
- **Severity:** P2
- **Description:** `ValidationDone` is defined and dispatched identically to
  `ModelsFetched`, but nothing emits it in production (only one test uses it).
  This duplication is confusing and adds surface area for bugs.
- **Suggested fix:** Remove `ValidationDone`, migrate the single test to
  `ModelsFetched`, and delete the duplicate handler arm.

---

### P3 — Public env-only builders are still exposed

- **File:** `crates/runie-agent/src/lib.rs`, `crates/runie-agent/src/subagent.rs`,
  `crates/runie-provider/src/lib.rs`
- **Lines:** `runie-agent/src/lib.rs` 45–49 (`build_provider_with_warning`);
  `runie-agent/src/subagent.rs` 33–55 (`run_subagent`); `runie-provider/src/lib.rs`
  239–246 (`switch_provider`)
- **Severity:** P3
- **Description:** After the move to config-aware builders, the legacy env-only
  variants remain public. `run_subagent` uses `Config::default()` and will fail
  for providers whose keys were saved during onboarding; `switch_provider` is
  unused and also env-only. These are footguns for future callers.
- **Suggested fix:** Mark them `#[deprecated]` or make them test-only / crate-
  private. Remove `switch_provider` if it has no callers.

### P3 — Duplicate providers-dialog handlers are dead code

- **File:** `crates/runie-core/src/update/login_flow.rs`
- **Lines:** 14–92 (`providers_event` and helpers)
- **Severity:** P3
- **Description:** `providers_event`, `open_providers_dialog`,
  `providers_select_model`, and `providers_disconnect` are marked
  `#[allow(dead_code)]`. The live handling is in
  `crates/runie-core/src/update/dialog/toggle.rs`. The duplicated logic can
  drift and mislead maintainers.
- **Suggested fix:** Delete the dead functions from `update/login_flow.rs`.

### P3 — `add_error` does not reset the streaming buffer

- **File:** `crates/runie-core/src/update/agent/core.rs`
- **Lines:** 331–361 (`add_error`)
- **Severity:** P3
- **Description:** `add_error` clears streaming/turn flags and timers but does
  not reset `state.agent.streaming_buffer`. A mid-stream error could leave a
  stale tail that bleeds into the next turn until `set_thinking` resets it.
- **Suggested fix:** Call `self.agent.streaming_buffer.reset()` in `add_error`.

### P3 — Linter/file-length risks

- **File:** `crates/runie-core/src/update/login_flow.rs`,
  `crates/runie-core/src/dialog/panel.rs`,
  `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- **Severity:** P3
- **Description:** `update/login_flow.rs` is 460 lines, `dialog/panel.rs` is
  494 lines, and `session/mod.rs` is 470 lines — all close to the 500-line
  file limit enforced by `crates/runie-core/build.rs`. Several functions in
  `update/login_flow.rs` (e.g. `login_flow_save`, `pop_login_panel_or_close`,
  `take_or_create_login_stack`) are also approaching the 40-line function
  limit.
- **Suggested fix:** Refactor before adding new onboarding panels or state
  transitions; run `cargo build` after any change to verify the heuristics
  still pass.

---

## Test gaps

The following scenarios have no automated coverage and should be added
(preferably Layer 2 unit tests, plus one Layer 3 render test where noted):

1. **Stale validation response race:** send `ValidationFailed`/`ModelsFetched`
   for the previous provider while the flow is validating a different provider;
   assert it is ignored.
2. **Startup with saved provider:** write a provider to the test config path,
   create `AppState`, and assert the login dialog does not auto-open.
3. **`/new` after onboarding:** complete the login flow, run `/new`, and assert
   the active provider/model are preserved.
4. **Agent loop config refresh:** simulate onboarding, then verify that a
   subsequent turn can build the provider using the newly saved key (e.g. by
   reloading config inside the turn or by a targeted integration test).
5. **Empty env var fallback:** set an empty `*_API_KEY` env var, save a key via
   onboarding, and assert the saved key is used.
6. **Malformed config handling:** write invalid TOML to the config path and
   assert a warning/error is surfaced.
7. **Custom base_url onboarding:** if/when custom base URLs are supported in
   the UI, add coverage that the saved base URL is used for validation and
  provider building.

---

## Files scanned (non-exhaustive)

- `crates/runie-core/src/update/login_flow.rs`
- `crates/runie-core/src/login_flow/{mod,state,panels,validation}.rs`
- `crates/runie-core/src/login_config.rs`
- `crates/runie-core/src/providers_dialog.rs`
- `crates/runie-core/src/update/dialog/{form,panel,toggle,open}.rs`
- `crates/runie-core/src/dialog/panel.rs`
- `crates/runie-core/src/update/agent/core.rs`
- `crates/runie-core/src/config.rs`
- `crates/runie-core/src/state/session.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/app_init.rs`
- `crates/runie-tui/src/effects/{login,subagent}.rs`
- `crates/runie-agent/src/{lib,subagent,turn}.rs`
- `crates/runie-provider/src/{lib,config}.rs`
