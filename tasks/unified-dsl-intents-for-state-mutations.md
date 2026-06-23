# Unified DSL intents for all state mutations

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, declarative-actor-dsl
**Blocks**: none

## Description

Slash commands, palette commands, dialog actions, and keybindings currently mix direct state mutation with event emission. Standardize every user-facing action on a small set of typed intents that route to the owning actor.

Current problems:
- `/theme` assigns `state.config.theme_name` directly.
- `/trust` emits a `ModelConfigEvent` that is then handled by mutating state.
- `/model` calls `state.switch_model`, which mutates config and fires a `ConfigMsg`.
- Settings dialog toggles call `state.toggle_read_only` directly.
- Dialog panel actions sometimes emit events, sometimes call helper methods, sometimes mutate state inline.

## Acceptance criteria

- [ ] All slash commands in `commands/dsl/handlers/` emit intent events only; they do not call `AppState` mutating helpers.
- [ ] All palette items emit intent events only.
- [ ] All dialog actions (`ItemAction::Emit`) emit intent events only.
- [ ] Settings/providers dialog actions emit `ConfigMsg`-family intents or `ConfigActorHandle` helpers.
- [ ] Login flow actions emit `ConfigMsg`/`TurnMsg`/`SessionMsg` intents as appropriate.
- [ ] Keybindings map to intent events, not to direct handler calls (where feasible).
- [ ] `CommandResult` variants that trigger side-effects (`Message`, `Warning`, etc.) are replaced or augmented with intent events.
- [ ] A single `Intent` enum (or dedicated intent sub-enums per actor) is the only thing produced by the DSL layer.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `every_command_emits_intent` — introspection / pattern test that no command helper mutates `AppState` directly.

### Layer 2 — Event Handling
- [ ] `theme_command_emits_set_theme` — `/theme runie` produces `SetTheme` intent.
- [ ] `trust_command_emits_set_trust` — `/trust` produces `TrustMsg::SetTrust` intent.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `dsl_intent_smoke_test` — run a sequence of slash commands and verify only intent events are produced.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/` — all handler files.
- `crates/runie-core/src/commands/registry.rs` — ensure commands can emit events.
- `crates/runie-core/src/event/` — add intent variants.
- `crates/runie-core/src/update/command.rs` — command dispatcher routes intents.
- `crates/runie-core/src/keybindings/` — keybinding actions emit intents.
- `crates/runie-core/src/dialog/` — panel builders emit intents.

## Notes

- This is a cross-cutting cleanup task. It depends on the actor taxonomy from `event-taxonomy-for-actor-state-sync`.
- Not every helper needs to become an event; pure computations can stay as functions. Only state mutations must become intents.
