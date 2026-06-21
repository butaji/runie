# Consolidate slash command handlers into a single dispatcher

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Slash commands are dispatched in two places that should be one:

- `crates/runie-core/src/commands/registry.rs` builds a typed `CommandRegistry` of named commands and their argument forms. This is the *declarative* half.
- `crates/runie-core/src/update/command.rs::handle_command_event` runs the imperative half: it pattern-matches every `RunXCommand` variant, mutates `AppState`, and emits follow-up events. Same flows exist in `update/dialog/` for the form-driven variants (`CommandFormInput`, `CommandFormSubmit`).

This duplication is the same shape flagged by `aggressive-event-consolidation`. The result: each new slash command requires a handler in three places (registry entry, dispatcher arm, and dialog form), and bugs hide in the gap between them.

## Acceptance Criteria

- [ ] One canonical handler per slash command. The dispatcher in `update/command.rs` calls into `commands/registry.rs` instead of pattern-matching on `Event::RunXCommand`.
- [ ] Dialog form commands (`CommandFormSubmit`) route through the same handler with a different argument source (form fields instead of parsed tokens).
- [ ] No slash command requires more than one new code path to add.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux validation: every `/command` documented in `README.md` (load, save, fork, model, theme, prompt, skill, login, logout, compact, thinking, palette, name, etc.) still works.

## Tests

### Layer 1 — State/Logic
- [ ] For each command: a unit test that feeds the canonical `RunXCommand` event into `AppState::update` and asserts the resulting state transition + emitted events.
- [ ] `form_submit_equivalent_to_run_command` — for each command exposed as a form, `CommandFormSubmit` with the same arguments produces identical state changes to `RunXCommand`.

### Layer 2 — Event Handling
- [ ] `slash_model_switches_provider` — `/model anthropic/claude-sonnet-4-6` updates `state.config.model` and emits `SwitchModel` event.

### Layer 3 — Rendering
- N/A — command palette and form rendering live in `popups/`.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/update/command.rs`
- `crates/runie-core/src/update/dialog/`
- All `RunXCommand` event producers (currently spread across `update/dialog/`, keymap, and the effect handlers).

## Notes

- Coordinate with `audit-borrow-workarounds` (P3) — slash commands are a known source of `take()`/`mem::take()` on `open_dialog` to work around the borrow checker.
- Coordinate with `split-files-at-limit-round-2` — `update/command.rs` and `update/dialog/` are large.
