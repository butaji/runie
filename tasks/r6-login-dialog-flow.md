# Login Dialog Flow

**Status**: done
**Milestone**: R6
**Category**: Configuration

## Description

Redesign `/login` as a multi-step dialog flow that guides users through provider setup.
Eliminate the separate `auth.json` file — all credentials live in the single `~/.runie/config.toml`.

Flow: `/login` → choose provider → enter API key → validate key with live API call → multi-select available models → save to config.

## Acceptance Criteria

- [x] `/login` opens a dialog with a picker of known providers (anthropic, openai, minimax, etc.)
- [x] Selecting a provider opens an API key input panel
- [x] Submitting a key triggers a live validation call to the provider's `/models` endpoint
- [x] Validation shows a loading state, then either an error panel or a model multi-select panel
- [x] Model multi-select uses toggle items; user selects which models to enable
- [x] Saving writes `[model_providers.{name}]` with `base_url`, `api_key`, and `models` to `config.toml`
- [x] `/logout` opens a dialog listing configured providers; selecting one removes it from config
- [x] No `auth.json` is read or written by `/login` or `/logout`
- [x] Provider resolver reads API keys from `config.toml` `[model_providers]` section

## Tests

### Layer 1 — State/Logic
- [x] `provider_registry_lists_known_providers` — registry returns all known providers with correct metadata
- [x] `provider_registry_find_by_name` — find provider by key returns correct metadata
- [x] `login_flow_state_transitions` — pure state machine transitions correctly through steps
- [x] `login_flow_builds_provider_picker` — provider picker panel has correct items
- [x] `login_flow_builds_key_input_panel` — key input panel has form field for the provider
- [x] `login_flow_builds_model_selector` — model selector has toggle items for each model
- [x] `config_save_provider_writes_toml` — saving provider config writes correct TOML structure
- [x] `config_remove_provider_deletes_section` — logout removes provider from TOML

### Layer 2 — Event Handling
- [x] `login_command_opens_provider_picker` — `/login` opens PanelStack with provider picker
- [x] `select_provider_pushes_key_input` — selecting provider in login flow opens key input
- [x] `submit_key_triggers_validation` — submitting key emits validation event
- [x] `validation_success_shows_models` — validation result event rebuilds dialog with model toggles
- [x] `validation_failure_shows_error` — validation error shows error panel with retry
- [x] `save_writes_config_and_closes` — save event writes config and closes dialog
- [x] `cancel_closes_dialog` — cancel event closes dialog without writing config
- [x] `form_button_activated_by_enter` — Enter on a button activates it
- [x] `form_button_activated_by_accelerator` — Accelerator key (e.g. `C` for `_Cancel`) activates button when not on a form field
- [x] `form_field_submit_still_builds_form_values` — Enter on a form field still submits the form

### Layer 3 — Rendering
- [x] `provider_picker_renders` — TestBackend renders provider picker correctly
- [x] `key_input_panel_renders` — TestBackend renders key input form panel
- [x] `model_selector_renders_toggles` — TestBackend renders model toggles
- [x] `form_buttons_render_inline_bottom_right` — Buttons render on same line, right-aligned, no borders, bg-colored
- [x] `form_active_button_uses_accent_color` — Active button uses accent color background

### Layer 4 — Smoke
- [x] `e2e_login_flow_opens_and_cancel_works` — `rexpect` PTY test: `/login` → navigate to API key form → cancel → no panic/stuck timers

## Notes

- Provider metadata (base URLs, env vars) is stored in a static registry in `provider_registry.rs`
- Validation uses `GET {base_url}/models` for OpenAI-compatible providers
- The `runie-term` event loop spawns async validation tasks and sends results back via event channel
- `auth.json` and `AuthStorage` are deprecated but kept for backward compatibility; new code uses config.toml only
