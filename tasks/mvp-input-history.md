# Input history

**Status**: done

**Milestone**: MVP

**Category**: Input & Commands

## Description

Command history navigation.

## Acceptance Criteria

- [x] Up/Down arrows for history (history_prev/history_next)
- [ ] Persistent history across sessions (deferred to R1)
- [ ] Search/filter history (deferred to R1)

## Tests

### Layer 1 — State/Logic
- [x] `history_prev_moves_up` — cycles to previous input
- [x] `history_next_moves_down` — cycles to next input
- [x] `history_wraps_at_bounds` — stops at first/last entry

### Layer 2 — Event Handling
- [x] `up_arrow_triggers_history_prev` — crossterm Up key
- [x] `down_arrow_triggers_history_next` — crossterm Down key

### Layer 3 — Rendering
N/A (input state only)

## Notes

- Persistent history and search/filter are deferred to R1 (see `r1-input-history-persistence`)
