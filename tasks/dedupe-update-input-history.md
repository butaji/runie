# Dedupe history navigation handlers

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/input/mod.rs` defines `handle_history_prev` and `handle_history_next` with mirrored structure: both check `vim_nav_mode`, then path-completion mode, then multi-line input, then call the corresponding history/nav method. Adding a new input mode requires edits in two places.

## Acceptance Criteria

- [ ] A helper returns the chosen navigation action based on current input state.
- [ ] `handle_history_prev` and `handle_history_next` map the action to up/down.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `history_nav_action_selects_by_mode` — helper returns the correct action for vim/path/multi-line modes.

### Layer 2 — Event Handling
- [ ] `history_prev_moves_up` — key event still navigates history backward.
- [ ] `history_next_moves_down` — key event still navigates history forward.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/update/input/mod.rs`

## Notes

Keep behavior identical; the refactor is purely structural.
