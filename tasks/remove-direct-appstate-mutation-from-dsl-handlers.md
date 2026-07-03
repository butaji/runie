# Remove direct `AppState` mutation from DSL command handlers

**Status**: done
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

DSL command handlers in `crates/runie-core/src/commands/dsl/` are defined as `fn(&mut AppState, &str) -> CommandResult` and mutate `AppState` directly. For example, `handle_new` in `commands/dsl/handlers/session/mod.rs:104` clears `state.session_mut().messages` and input fields synchronously. This bypasses the actor-message → state transition → event emission flow that `AGENTS.md` and the SSOT ADR require.

## Acceptance Criteria

- [x] Change the handler signature from `fn(&mut AppState, &str)` to a form that returns events/intents.
- [x] Refactor every handler in `commands/dsl/handlers/` to emit `CommandResult::Events(...)` or send an actor message instead of mutating `AppState`.
- [x] Update `UiActor::apply_event` (or equivalent) to apply the resulting events.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dsl_handler_returns_events_not_mutation` — handlers produce `Vec<Event>` for a given input.
- [x] `command_registry_rejects_mutating_handlers` — registry no longer accepts `fn(&mut AppState, _)`.

### Layer 2 — Event Handling
- [x] `ui_actor_applies_dsl_events` — `UiActor` applies handler-emitted events to `AppState` the same way actor facts are applied.

### Layer 3 — Rendering
- [x] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `slash_commands_emit_expected_events` — replay tests verify `/new`, `/save`, `/model`, etc. emit the expected event sequence.

### Live Tmux Testing Session
- [x] Run `/new`, `/save`, `/model`, and `/session` in the TUI and verify they behave identically to before.

## Files touched

- `crates/runie-core/src/commands/dsl/command.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/run.rs`
- `crates/runie-core/src/commands/dsl/handlers/model.rs`
- `crates/runie-core/src/commands/dsl/handlers/*.rs`
- `crates/runie-core/src/ui_actor/mod.rs` (or `crates/runie-tui/src/ui_actor/mod.rs` if that owns application)

## Notes

- Supersedes the remaining work from `remove-direct-appstate-mutation-from-tui-handlers.md`.
- The synchronous "test fallback" path in handlers should be removed; tests should drive handlers through events.
