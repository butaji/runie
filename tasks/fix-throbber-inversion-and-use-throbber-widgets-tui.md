# Fix throbber inversion and use `throbber_widgets_tui`

## Status

`done`

## Description

The status bar extracts braille symbols manually to mirror an inverted spinner. Drop the inversion and use `throbber_widgets_tui::Throbber` directly.

## Implementation

### `crates/runie-tui/src/status_bar.rs`
- Replaced manual symbol extraction (`throbber_current_symbol`) with `Throbber` widget from `throbber-widgets-tui`
- Layout split into `[spinner_area][text_area]` using `ratatui::layout::Layout`
- `f.render_stateful_widget(throbber_widget, spinner_area, throbber)` replaces the `set_span` overlay hack
- Removed the hand-rolled `BRAILLE_SIX` char array

### `crates/runie-tui/src/ui.rs`
- Simplified throbber initialization: removed backward-index calculation, uses forward index matching the widget

### `crates/runie-core/src/model/state/domain_ops.rs`
- `spinner_frame()` now uses forward indexing (matching Throbber widget behavior)

### `crates/runie-tui/src/tests/render_actor.rs`
- Updated `snapshot_spinner_frame_captured` test to expect `'⠾'` (frame 5 forward) instead of `'⠷'` (frame 5 backward)

## Acceptance criteria

1. **Unit tests** — Spinner frames progress in the natural order. ✓
2. **E2E tests** — Rendering snapshots show a correctly animated throbber. ✓ (709 tests pass)
3. **Live tmux tests** — Watch the spinner during a streaming turn in tmux. ✓

## Tests

### Unit tests
- Frame index advances normally.

### E2E tests
- Snapshot test of status bar during streaming.

### Live tmux tests
- Submit a prompt and observe the spinner animation.
