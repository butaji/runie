# Refine permission dialog key handling

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: wire-user-permission-rules-into-agent-gate

## Description

The permission dialog treats every key except `y`/`Y`/`a`/`A` as deny. Navigation keys (Esc, Enter, arrows) and unrelated characters accidentally deny the request, which is poor UX and may hide the dialog-focus bug.

## Root Cause

`crates/runie-core/src/update/input/mod.rs` (`permission_input_event`) has a coarse match that maps all non-allow keys to deny.

## Acceptance Criteria

- [ ] `y`/`Y` allows once; `a`/`A` allows always.
- [ ] `n`/`N` explicitly denies.
- [ ] Esc/Back/arrow keys are consumed as no-ops while the dialog is open (do not deny).
- [ ] Keys that are not dialog actions are not routed to the input box.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux permission dialog does not accidentally deny on arrow/Esc keys.

## Tests

### Layer 2 — Event Handling
- [ ] `esc_during_permission_dialog_is_noop` — Esc while a permission request is open does not emit deny.
- [ ] `n_key_denies` — `n` explicitly denies.

### Layer 3 — Rendering
- [ ] `permission_dialog_shows_focus` — `TestBackend` highlights the focused choice.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_permission_esc_no_deny` — live tmux script opens the dialog, presses Esc, and asserts the dialog stays open.

## Files touched

- `crates/runie-core/src/update/input/mod.rs`
- `crates/runie-core/src/update/dialog/router.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This should be fixed after the dialog-focus bug, since correct key routing is required first.
