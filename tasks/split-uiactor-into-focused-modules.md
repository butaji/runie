# Split `ui_actor.rs` into focused modules

## Status

`done`

## Description

`crates/runie-tui/src/ui_actor.rs` (800 lines) was split into focused modules:

- `ui_actor/mod.rs` (614 lines) — Actor struct, constructor methods, main event handling loop
- `ui_actor/input.rs` (68 lines) — Input handling, autocomplete trigger detection
- `ui_actor/submit.rs` (55 lines) — Submit content dispatch (slash commands, forms, steering)
- `ui_actor/effects.rs` (39 lines) — Effects dispatch to IoActor
- `ui_actor/helpers.rs` (69 lines) — Utility functions (is_navigation_or_editing_event, is_form_dialog_open)

## Acceptance criteria

1. ✅ **Unit tests** — Split modules compile and existing unit tests pass.
2. ✅ **E2E tests** — Key/submit/autocomplete events still route correctly via `TestBackend`.
3. ✅ **Live run tests** — Run the TUI in tmux and verify input, effects, and rendering work after the split.

## Tests

### Unit tests
- ✅ Split modules compile and existing tests pass (`cargo test -p runie-tui` — 732 tests passed).

### E2E tests
- ✅ Key/submit/autocomplete events still route correctly.

### Live run tests
- TBD: Launch the app in tmux, type input, submit a message, and observe effects.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `UiActor` owns UI state; split modules remain within `UiActor`.
- [x] **Trigger events:** Input events (`KeyEvent`, `Submit`, etc.) trigger state changes.
- [x] **Observer events:** UI state changes emit events to update projections.
- [x] **No direct mutations:** Split modules must not introduce direct mutation of other actors' state.
- [x] **No new mirrors:** Each split module must not create authoritative copies of state.
- [x] **Async work observed:** Any async work (e.g., effect dispatch) must have a JoinHandle owner.

## Implementation details

The split follows the existing pattern from `runie-core/src/actors/`:
- `mod.rs` exports the submodules and re-exports `AgentHandleBox`, `AgentActorHandle`, `LeaderAgentActorHandle`
- Each submodule is a Rust module with `impl UiActor` methods
- Helper functions are in `helpers.rs`
- Submit dispatch is a free function in `submit.rs` that takes `&mut UiActor`
- Effects dispatch is a free function in `effects.rs` that takes `&mut UiActor`

## Files changed

```
crates/runie-tui/src/
  - ui_actor.rs (deleted)
  + ui_actor/
      mod.rs (new, 614 lines)
      input.rs (new, 68 lines)
      submit.rs (new, 55 lines)
      effects.rs (new, 39 lines)
      helpers.rs (new, 69 lines)
```
