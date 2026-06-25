# Dedupe history navigation handlers

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/input/mod.rs` defines `handle_history_prev` and `handle_history_next` with mirrored structure: both check `vim_nav_mode`, then path-completion mode, then multi-line input, then call the corresponding history/nav method. Adding a new input mode requires edits in two places.

## Acceptance Criteria

- [x] A helper returns the chosen navigation action based on current input state.
- [x] `handle_history_prev` and `handle_history_next` map the action to up/down.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `history_nav_mode_selects_by_mode` — helper returns the correct action for path/multi-line/plain modes.
  - `history_nav_mode_selects_path_complete_when_suggestions_open`
  - `history_nav_mode_selects_cursor_when_multiline_input`
  - `history_nav_mode_selects_history_when_plain_input`

### Layer 2 — Event Handling
- [x] `history_prev_moves_up` — key event still navigates history backward.
- [x] `history_next_moves_down` — key event still navigates history forward.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files touched

- `crates/runie-core/src/update/input/mod.rs` — added `HistoryNavMode` enum and `get_history_nav_mode` helper
- `crates/runie-core/src/update/input/tests.rs` — added Layer 1 tests

## Notes

Keep behavior identical; the refactor is purely structural.
