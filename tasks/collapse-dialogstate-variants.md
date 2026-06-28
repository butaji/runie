# Collapse `DialogState` variants

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`DialogState` contains overlapping variants and ad-hoc routing that make input handling hard to follow. Refactor it to a small, mutually exclusive set of states (e.g., `Idle`, `Prompting`, `Confirming`, `Streaming`) with explicit transitions.

## Acceptance Criteria

- [ ] `DialogState` has no more than four top-level variants.
- [ ] Transitions are implemented as a pure state machine, not scattered `if` branches.
- [ ] No duplicate data is stored across variants.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dialog_transitions_are_valid` — invalid transitions are rejected by the state machine.
- [ ] `dialog_prompt_data_unique` — only the active variant carries prompt-specific data.

### Layer 2 — Event Handling
- [ ] `escape_cancels_dialog` — Escape returns to `Idle` from any active dialog state.
- [ ] `enter_confirms_dialog` — Enter confirms only when in `Confirming`/`Prompting`.

### Layer 3 — Rendering
- [ ] `dialog_renders_per_state` — each state renders the expected `TestBackend` buffer.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — dialog state is local TUI state.

## Files touched

- `crates/runie-tui/src/dialog.rs`
- `crates/runie-tui/src/app.rs`
- `crates/runie-tui/src/handler.rs`

## Notes

- If confirmation and prompting differ only by a callback, consider a single `Active` variant holding a payload enum.
- Preserve existing keyboard shortcuts; do not change UX while simplifying internals.
