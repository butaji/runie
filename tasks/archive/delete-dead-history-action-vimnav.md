# Delete unused `HistoryAction::VimNav` and `dir` parameter

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/update/input/mod.rs` declares:

```rust
enum HistoryAction {
    VimNav(bool),     // line 116 — never constructed
    // ...
}

fn navigate_history(state: &mut AppState, dir: Direction) -> HistoryAction {
    // `dir` is never used in the body — line 122 warning
}
```

The `VimNav` variant is dead; `dir` is computed by callers and then ignored. Either wire `dir` into a real navigation step (most likely: make `HistoryAction` carry a `Direction` so `navigate_history` actually moves through history) or delete both the variant and the parameter.

## Acceptance Criteria

- [ ] One of:
  - `HistoryAction` is extended to carry a `Direction` and `navigate_history` actually consumes `dir` (preferred — restores intent).
  - `VimNav(bool)` removed and `navigate_history` no longer takes `dir`.
- [ ] No new warnings about unused variables or dead enum variants.
- [ ] Live tmux validation: history recall (`Up`/`Down` keys when the input box is focused) still scrolls through past entries.

## Tests

### Layer 1 — State/Logic
- [ ] `history_navigation_steps_through_entries` — feed three history entries, call `navigate_history(Direction::Prev)` twice, assert the input reflects the second-to-last entry.

### Layer 2 — Event Handling
- [ ] `event_history_prev_updates_input` — `AppState::update(Event::HistoryPrev)` after `HistoryAppend` populates the input with the most recent entry.

### Layer 3 — Rendering
- N/A — input box rendering is unchanged.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-core/src/update/input/mod.rs`

## Notes

- Pair with `delete-dead-theme-async-loaders` and `delete-dead-tuple-actor-handles-fields`.
