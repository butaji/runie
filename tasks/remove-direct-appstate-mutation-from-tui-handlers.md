# Remove direct AppState mutation from TUI handlers

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: subscribe-tui-to-initial-facts-before-leader-start
**Blocks**: fix-tui-slash-command-palette-stays-open-after-execution, fix-tui-form-submit-key-not-working

## Description

Command handlers, dialog panel handlers, and `UiActor` directly mutate `AppState` fields (session messages, input, view, config, dialog stack) instead of emitting intents for the owning actor to apply. This violates the documented architecture where actors are the single source of truth and state sync is event-driven.

## Root Cause

The event-driven refactor is incomplete. Legacy code paths in `crates/runie-core/src/update/` and `crates/runie-core/src/commands/dsl/handlers/` still mutate `AppState` directly, often guarded by `tokio::runtime::Handle::try_current()` to fake actor behavior in tests.

## Acceptance Criteria

- [ ] No handler, command, dialog, or render function mutates actor-owned `AppState` fields directly.
- [ ] All state changes flow through the owning actor and are applied via emitted facts.
- [ ] The `tokio::runtime::Handle::try_current()` branching in `submit_user_message`, `run_bash_command`, and similar paths is removed.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux smoke tests for `/save`, `/session`, `/history`, and input still work.

## Tests

### Layer 1 — State/Logic
- [ ] `no_direct_appstate_mutation_in_handlers` — static check (grep) that no handler file assigns to `state.*` actor-owned fields.

### Layer 2 — Event Handling
- [ ] `submit_user_message_emits_intent_not_mutation` — `Event::Submit` results in a `TurnMsg::SubmitUserMessage` intent, not a direct state change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_save_session_still_works` — live tmux regression test.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/update/dialog/panel_handler.rs`
- `crates/runie-core/src/update/input/submit.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/model/app_state.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is a large architectural cleanup. It may need to be split further once the scope is measured.
- Fixing this will make many of the current live-testing bugs (palette overlay, form state, history) easier to reason about.
