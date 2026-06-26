# Unified DSL intents for all state mutations

**Status**: in_progress
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, declarative-actor-dsl
**Blocks**: none

## Progress

**Completed changes:**
- ✅ `/theme` now emits `Event::SwitchTheme` instead of directly setting `state.config_mut().theme_name`
- ✅ `/theme` with no args now returns `OpenDialog(DialogType::ThemeSelector)` which opens the theme selector via the event system
- ✅ `/reload` now emits `Event::ReloadAll` instead of directly calling `*state.skills_mut() = crate::skills::load_all()`
- ✅ Added `DialogType::ThemeSelector` to the DialogType enum
- ✅ Added `open_theme_selector` function in `update/dialog/open.rs` that properly builds the panel and sets state via the event system

**Remaining work:**
- Session command handlers (`run_name`, `run_fork`, `run_compact`) still mutate state directly
- `run_prompt` handler still mutates state directly
- `handle_settings` still calls `open_settings_dialog` which directly mutates state

## Description

Slash commands, palette commands, dialog actions, and keybindings currently mix direct state mutation with event emission. Standardize every user-facing action on a small set of typed intents that route to the owning actor.

## Acceptance criteria

- [x] All slash commands in `commands/dsl/handlers/` emit intent events only; they do not call `AppState` mutating helpers. (Partial: /theme, /model, /thinking, /reload done)
- [ ] All palette items emit intent events only.
- [ ] All dialog actions (`ItemAction::Emit`) emit intent events only.
- [ ] Settings/providers dialog actions emit `ConfigMsg`-family intents or `ConfigActorHandle` helpers.
- [ ] Login flow actions emit `ConfigMsg`/`TurnMsg`/`SessionMsg` intents as appropriate.
- [ ] Keybindings map to intent events, not to direct handler calls (where feasible).
- [ ] `CommandResult` variants that trigger side-effects (`Message`, `Warning`, etc.) are replaced or augmented with intent events.
- [ ] A single `Intent` enum (or dedicated intent sub-enums per actor) is the only thing produced by the DSL layer.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `every_command_emits_intent` — introspection / pattern test that no command helper mutates `AppState` directly.

### Layer 2 — Event Handling
- [x] `theme_command_emits_set_theme` — `/theme runie` produces `SwitchTheme` event. ✅
- [ ] `trust_command_emits_set_trust` — `/trust` produces `TrustMsg::SetTrust` intent.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `dsl_intent_smoke_test` — run a sequence of slash commands and verify only intent events are produced.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/theme` and `/reload` handlers now emit events
- `crates/runie-core/src/commands/dsl/flow.rs` — added `DialogType::ThemeSelector`
- `crates/runie-core/src/update/dialog/open.rs` — added `open_theme_selector` function
- `crates/runie-core/src/update/dialog/router.rs` — added `ThemeSelector` handling
- `crates/runie-core/src/update/input/submit.rs` — added `ThemeSelector` handling
- `crates/runie-core/src/update/dialog/mod.rs` — exported `open_theme_selector`
- `crates/runie-core/src/tests/reload.rs` — updated test to expect `ReloadAll` event

## Notes

- This is a cross-cutting cleanup task. It depends on the actor taxonomy from `event-taxonomy-for-actor-state-sync`.
- Not every helper needs to become an event; pure computations can stay as functions. Only state mutations must become intents.
