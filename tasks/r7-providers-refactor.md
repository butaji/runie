# Providers Refactor

**Status**: done
**Milestone**: R7
**Category**: Configuration

## Description

Replace `/login` and `/logout` commands with `/providers` as the unified entry point for provider management. The providers dialog is the single interface for:
- Viewing configured providers
- Selecting active models
- Adding new providers (via login flow)
- Disconnecting providers

### Key Flow Changes

**Before**: `/login` → provider picker → API key → auto-activate first model → close

**After**: `/providers` → + Add provider → login flow → Save → providers dialog → **select active model** → close

The user now has control over which model to activate after connecting a provider.

## Changes Made

### Removed Commands
- `/login` - replaced by `/providers` dialog
- `/logout` - replaced by `/providers` dialog

### Added/Updated Commands
- `/providers` - main command for provider management
- `/provider` - alias for `/providers`

### Architecture Changes
- `providers_dialog.rs` - unified dialog for all provider operations
- `login_flow.rs` - guided multi-step flow for adding providers (accessed via providers dialog)
- `update/mod.rs` - modified `login_flow_save` to restore providers dialog after save instead of auto-activating

### Backward Compatibility
- `RunLoginCommand` and `RunLogoutCommand` events still work for scripting
- `run_login_command` redirects to providers dialog for guided flow
- `run_logout_command` removes provider from config

## Acceptance Criteria

- [x] `/providers` command opens the providers dialog
- [x] `/provider` alias works
- [x] Providers dialog shows all configured providers with their models
- [x] "+ Add provider" starts the login flow
- [x] Login flow completion restores providers dialog (not auto-activating)
- [x] User can choose which model to activate from providers dialog
- [x] "Disconnect" removes a provider from config.toml
- [x] Disconnecting clears active provider if no other providers exist
- [x] Login flow cancel returns to providers dialog

## Tests

### Layer 1 — State/Logic
- [x] `providers_command_opens_dialog` — /providers opens the providers dialog
- [x] `login_flow_state_machine_provider_picker` — flow starts at provider picker
- [x] `login_flow_state_machine_key_input` — selecting provider goes to key input
- [x] `login_flow_state_machine_model_select` — submitting key goes to model select
- [x] `login_flow_toggle_model` — toggling model updates selection
- [x] `login_flow_with_unknown_provider` — handles unknown providers gracefully

### Layer 2 — Event Handling
- [x] `slash_providers_opens_dialog` — typing /providers and Enter opens the dialog
- [x] `slash_provider_alias_opens_dialog` — /provider alias opens the dialog
- [x] `providers_add_starts_login_flow` — clicking "Add" starts the login flow
- [x] `login_flow_save_shows_providers_dialog` — save restores providers dialog
- [x] `login_flow_save_does_not_auto_activate_model` — save doesn't auto-activate
- [x] `login_flow_save_allows_model_selection` — user can select model after save
- [x] `login_flow_save_allows_model_selection_from_multiple` — select from multiple models
- [x] `login_flow_save_saves_config` — config is saved to file
- [x] `providers_select_model_switches_active_model` — selecting model updates config
- [x] `providers_select_model_closes_dialog` — selecting model closes dialog
- [x] `providers_select_model_records_usage` — usage is recorded in recent_models
- [x] `providers_disconnect_removes_provider` — disconnect clears current provider
- [x] `providers_disconnect_closes_dialog` — disconnect closes the dialog
- [x] `disconnect_active_provider_switches_to_another` — switch to remaining provider
- [x] `disconnect_clears_active_provider_when_no_other` — clear when no other providers
- [x] `login_flow_cancel_returns_to_providers_dialog` — cancel restores previous dialog
- [x] `providers_dialog_empty_state` — works with no providers configured

### Updated Tests (login_flow.rs)
- [x] `s6_save_before_fetch_then_fetch_is_ignored` — updated to expect providers dialog after save

### Smoke Tests
- [x] `tmux_login_logout_test.sh` — E2E test for /providers command
