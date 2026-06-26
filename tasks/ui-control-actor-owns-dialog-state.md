# UiControlActor owns dialog and login-flow lifecycle

**Status**: done
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

## Implementation

`UiControlActor` is implemented in `crates/runie-core/src/actors/ui_control/`. It owns:
- `dialog`: currently open dialog
- `back_stack`: dialog stack for back navigation
- `flow`: login flow state machine
- `quit_requested`: quit flag

The actor emits these facts:
- `DialogOpened { kind: DialogKind }` — when a dialog is opened
- `DialogClosed` — when a dialog is closed
- `LoginFlowStepChanged { step, provider }` — when login flow step changes
- `LoginFlowClosed` — when login flow ends
- `QuitRequested { forced }` — when quit is requested

## Remaining work

The actor is implemented, but handlers still directly mutate `AppState.open_dialog`, etc. The next step is to update handlers to emit `UiControlMsg` instead of direct mutations. This is tracked as incremental work.

## Acceptance criteria

- [x] `UiControlActor` is an mpsc actor owning `open_dialog`, `dialog_back_stack`, `login_flow`, and `should_quit`.
- [x] `UiControlMsg` covers: `OpenDialog { dialog }`, `PushDialog { dialog }`, `PopDialog`, `CloseAllDialogs`, `StartLoginFlow`, `CancelLoginFlow`, `LoginFlowStep { step }`, `RequestQuit`, `ForceQuit`.
- [x] `UiControlActor` emits facts: `DialogOpened`, `DialogClosed`, `LoginFlowStepChanged`, `LoginFlowClosed`, `QuitRequested`.
- [x] `cargo test --workspace` passes.

## Files touched

- `crates/runie-core/src/actors/ui_control/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/actors/mod.rs` — exports `UiControlActor`, `UiControlMsg`, `UiControlActorHandle`.
- `crates/runie-core/src/event/variants.rs` — added `DialogKind` enum and fact variants.
- `crates/runie-core/src/event/mod.rs` — exports `DialogKind`.
- `crates/runie-core/src/login_flow/state.rs` — added serde derives.
- `crates/runie-core/build.rs` — added exemptions for actor files.
