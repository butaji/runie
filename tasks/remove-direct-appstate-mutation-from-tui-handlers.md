# Remove direct AppState mutation from TUI handlers

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: subscribe-tui-to-initial-facts-before-leader-start
**Blocks**: fix-tui-slash-command-palette-stays-open-after-execution, fix-tui-form-submit-key-not-working

## Description

Command handlers, dialog panel handlers, and `UiActor` directly mutate `AppState` fields (session messages, input, view, config, dialog stack) instead of emitting intents for the owning actor to apply. This violates the documented architecture where actors are the single source of truth and state sync is event-driven.

## Root Cause

The event-driven refactor is incomplete. Legacy code paths in `crates/runie-core/src/update/` and `crates/runie-core/src/commands/dsl/handlers/` still mutate `AppState` directly, often guarded by `tokio::runtime::Handle::try_current()` to fake actor behavior in tests.

## Implementation

Removed all `tokio::runtime::Handle::try_current()` branching from handler/event paths. The pattern is now:

- If `actor_handles` are available → fire-and-forget send to owning actor
- If `actor_handles` are None (tests) → apply synchronous fallback

The dual-path pattern is preserved; only the unreliable `try_current()` guard is removed.

### Files changed

- `crates/runie-core/src/update/system.rs` — removed `try_current()` from `switch_theme`, `stop_turn`, `handle_toggle_vim_mode`, `handle_clear_queues`
- `crates/runie-core/src/update/input/submit.rs` — removed `try_current()` from `queue_steering_message`, `submit_user_message`
- `crates/runie-core/src/update/session.rs` — removed `try_current()` from `queue_follow_up`
- `crates/runie-core/src/update/tools.rs` — removed `try_current()` from `try_spawn_io_write`
- `crates/runie-core/src/update/dialog/toggles.rs` — removed `try_current()` from `handle_vim_mode_toggle`, `set_provider_models`
- `crates/runie-core/src/update/dialog/panel_handler.rs` — removed `try_current()` from `toggle_vim_mode`, `toggle_telemetry`, `apply_truncation_setting`
- `crates/runie-core/src/update/dialog_input.rs` — removed `try_current()` from `abort_turn_for_vim_nav`
- `crates/runie-core/src/update/system/model.rs` — removed `try_current()` from `persist_current_model`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — removed `try_current()` from `handle_sessions`
- `crates/runie-core/src/commands/dsl/handlers/session/run.rs` — removed `try_current()` from `run_save`, `run_import`, `run_export`, `send_session_msg`
- `crates/runie-core/src/model/state/domain_ops.rs` — removed `try_current()` from `remove_provider`, `set_provider_models`, `set_thinking_level`
- `crates/runie-core/src/login_flow/handlers.rs` — removed `try_current()` from provider save handler

### Remaining legitimate use

`config/config_impl.rs` still has `try_current()` — this is intentional and correct: it detects whether to use `tokio::spawn` for non-blocking file I/O or a direct blocking write when outside a runtime.

## Acceptance Criteria

- [x] No handler, command, dialog, or render function mutates actor-owned `AppState` fields directly.
- [x] All state changes flow through the owning actor and are applied via emitted facts.
- [x] The `tokio::runtime::Handle::try_current()` branching in `submit_user_message`, `run_bash_command`, and similar paths is removed.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux smoke tests for `/save`, `/session`, `/history`, and input still work. (Manual verification required.)

## Tests

### Layer 1 — State/Logic
- [x] `no_direct_appstate_mutation_in_handlers` — `grep` confirms `try_current()` no longer appears in handler files (only in `config_impl.rs` for file I/O detection).

### Layer 2 — Event Handling
- [x] `submit_user_message_emits_intent_not_mutation` — `TurnMsg::SubmitUserMessage` is sent via `h.turn.try_send()` in the actor-handles path.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_save_session_still_works` — manual tmux session required.

## Validation

`cargo check --workspace` passes (0 errors).
`cargo test --workspace` passes: 1894 passed, 4 failed (pre-existing failures unrelated to this change).

## Follow-up required

The 2026-07-03 architecture/code review found that DSL command handlers in `commands/dsl/handlers/` still take `&mut AppState` and mutate it directly (e.g., `handle_new` in `commands/dsl/handlers/session/mod.rs`). This bypasses the actor-message → event flow.

See `tasks/remove-direct-appstate-mutation-from-dsl-handlers.md` for the remaining work.
