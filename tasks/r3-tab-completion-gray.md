# Tab Completion: Show Gray + Auto-complete Single Match

**Status**: done
**Milestone**: R3
**Category**: Input & Commands

## Description

Enhanced tab completion behavior for file/folder name autocompletion:
1. First Tab press: Shows the rest of the filename/folder in gray (ghost text)
2. Second Tab with single match: Auto-completes the word immediately
3. Second Tab with multiple matches: Cycles through options
4. Cursor movement, typing, backspace: Clears ghost completion

## Acceptance Criteria

- [x] First Tab shows ghost text (rest of filename in gray)
- [x] Second Tab with single match auto-completes (accepts ghost)
- [x] Second Tab with multiple matches cycles to next option
- [x] Cycling wraps around when reaching the end
- [x] Cursor movement clears ghost completion
- [x] Typing clears ghost completion
- [x] Backspace clears ghost completion
- [x] Submit (Enter) accepts ghost and includes in message

## Tests

### Layer 1 — State/Logic
- [x] `tab_second_press_single_match_completes` — verifies single match auto-completes
- [x] `cycling_changes_ghost` — verifies cycling state transitions
- [x] `tab_cycles_wraps_around` — verifies wrapping behavior
- [x] `new_prefix_resets_cycle` — verifies prefix change resets state
- [x] `ghost_cleared_after_completion` — verifies completion clears state

### Layer 2 — Event Handling
- [x] `cursor_movement_clears_ghost` — CursorLeft clears ghost
- [x] `cursor_right_clears_ghost` — CursorRight clears ghost
- [x] `enter_accepts_ghost` — Submit accepts ghost
- [x] `delete_word_clears_ghost` — DeleteWord clears ghost
- [x] `backspace_clears_ghost` — Backspace clears ghost
- [x] `typing_clears_ghost` — typing clears ghost
- [x] `tab_flash_on_empty_input` — empty input flashes
- [x] `tab_flash_on_no_match` — no match flashes

### Layer 3 — Rendering
- [x] Ghost text is rendered using `style_hint()` (existing behavior)

## Implementation Details

### Files Modified
- `crates/runie-core/src/update/tab_complete.rs` — Core tab completion logic
- `crates/runie-core/src/update/input.rs` — Added `clear_ghost()` calls to cursor movement
- `crates/runie-core/src/update/at_refs.rs` — Refactored to use `clear_ghost()`
- `crates/runie-core/src/tests/tab_complete.rs` — New TDD tests
- `crates/runie-core/src/tests/ghost_completion.rs` — New TDD tests

### Key Logic Changes

1. **Single match auto-complete**: When Tab is pressed with same prefix and only 1 match, `accept_ghost()` is called instead of cycling.

2. **Centralized clear_ghost()**: Created a centralized method that clears all ghost/completion state, called by cursor movement, typing, backspace, etc.

3. **Fixed bug**: Previous `clear_ghost()` used `take().is_some()` which mutated the Option even when short-circuit evaluation should have prevented it.

## Notes

- Out of scope: Testing with actual file system matches (tests use mocked state)
- The feature uses existing ghost text rendering mechanism
- Ghost is shown in gray using `style_hint()` style
