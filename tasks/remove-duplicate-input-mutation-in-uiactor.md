# Remove duplicate input mutation in UiActor

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: fix-tui-slash-command-palette-stays-open-after-execution

## Description

`UiActor::handle_input_event` both applies an input character directly to `AppState.input` and forwards `InputMsg::InsertChar` to `InputActor`. In the current live path `UiActor` never sees `Event::Input`, but the duplicate logic is a landmine: if `Event::Input` ever reaches `UiActor`, characters will be double-inserted.

## Root Cause

A legacy mutation path was left in place when `InputActor` became the single source of truth for input state.

## Acceptance Criteria

- [ ] `UiActor` never mutates `AppState.input` directly.
- [ ] All input changes are routed through `InputActor` and applied via `InputChanged` facts.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux input typing remains correct.

## Tests

### Layer 2 — Event Handling
- [ ] `input_event_only_routes_to_input_actor` — `Event::Input(c)` sent to `UiActor` produces exactly one `InputMsg::InsertChar` and no direct state mutation.

### Layer 3 — Rendering
- [ ] `input_character_renders_once` — `TestBackend` shows a typed char exactly once.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_type_hello_no_duplicates` — live tmux script types `hello` and asserts no double characters.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/actors/input/actor.rs`
- `crates/runie-core/src/model/app_state.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This cleanup is a prerequisite for safely routing more input events through `UiActor` if needed.
