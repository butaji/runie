# UiControlActor owns dialog and login-flow lifecycle

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: view-actor-owns-view-state, input-actor-owns-input-state, turn-actor-owns-agent-turn-state, remove-direct-appstate-mutations

## Description

Several UI-control fields are mutated by many handlers but were not assigned an actor in the first pass:

- `open_dialog` — which overlay is currently open.
- `dialog_back_stack` — global stack for Android-style back navigation.
- `login_flow` — login flow state machine.
- `should_quit` — main-loop quit flag.
- `view.input_receiver` — who receives keystrokes (chat input vs dialog form vs path completion). (This could also live in `ViewActor`; decide at implementation time.)

These are not domain state, but they are mutable and shared. A `UiControlActor` (or `DialogActor`) should own them so handlers only emit intents such as `OpenDialog`, `PushDialog`, `PopDialog`, `CloseAllDialogs`, `StartLoginFlow`, `CancelLoginFlow`, `Quit`.

Current violators:
- `update/dialog/*.rs` — openers, router, panel stack, toggle handlers set `open_dialog` and `dialog_back_stack`.
- `update/login_flow.rs` — builds/replaces/pops login panels and sets `login_flow`.
- `commands/dsl/handlers/system.rs` and others — `/quit` sets `should_quit`.
- `update/system.rs` — `handle_quit_event` sets `should_quit`.
- `update/dialog/open.rs` — sets `view.input_receiver`.

## Acceptance criteria

- [ ] `UiControlActor` is an mpsc actor owning `open_dialog`, `dialog_back_stack`, `login_flow`, and `should_quit`.
- [ ] `UiControlMsg` covers: `OpenDialog { dialog }`, `PushDialog { dialog }`, `PopDialog`, `CloseAllDialogs`, `StartLoginFlow`, `CancelLoginFlow`, `LoginFlowStep { step }`, `RequestQuit`, `ForceQuit`.
- [ ] `AppState.open_dialog`, `dialog_back_stack`, `login_flow`, `should_quit` are private; reads go through immutable accessors.
- [ ] `UiControlActor` emits facts: `DialogOpened`, `DialogClosed`, `LoginFlowStarted`, `LoginFlowStepChanged`, `LoginFlowClosed`, `QuitRequested`.
- [ ] `ViewActor` consumes `DialogOpened`/`DialogClosed` to update `input_receiver` if that field lives there.
- [ ] Handlers/commands no longer directly assign `open_dialog`/`dialog_back_stack`/`login_flow`/`should_quit`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `ui_control_actor_open_pushes_back_stack` — opening a dialog while another is open pushes the old one.
- [ ] `ui_control_actor_pop_restores_parent` — `PopDialog` restores the previous dialog.
- [ ] `ui_control_actor_start_login_flow_clears_dialog` — login flow starts with no other dialog open.

### Layer 2 — Event Handling
- [ ] `escape_key_sends_pop_dialog` — Escape sends `PopDialog` (or `CloseAllDialogs` at root).
- [ ] `quit_command_sends_request_quit` — `/quit` sends `RequestQuit`.

### Layer 3 — Rendering
- [ ] `dialog_opened_fact_renders_dialog` — `DialogOpened` causes the new dialog to render.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/ui_control/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — private control fields.
- `crates/runie-core/src/update/dialog/*.rs` — emit `UiControlMsg`.
- `crates/runie-core/src/update/login_flow.rs` — emit `UiControlMsg`.
- `crates/runie-core/src/update/system.rs` — quit handlers emit `UiControlMsg`.
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/quit` emits intent.
- `crates/runie-core/src/view_actor.rs` or `update/dialog/open.rs` — manage `input_receiver` via facts.

## Notes

- `UiControlActor` is the natural home for the global back-stack and overlay state. It may feel small, but without it these fields become the last direct-mutation holdouts.
- Coordinate with `view-actor-owns-view-state` on `input_receiver` ownership.
