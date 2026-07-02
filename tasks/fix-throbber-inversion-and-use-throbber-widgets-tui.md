# Fix throbber inversion and use `throbber_widgets_tui`

## Status

`todo`

## Description

The status bar extracts braille symbols manually to mirror an inverted spinner. Drop the inversion and use `throbber_widgets_tui::Throbber` directly.

## Acceptance criteria

1. **Unit tests** — Spinner frames progress in the natural order.
2. **E2E tests** — Rendering snapshots show a correctly animated throbber.
3. **Live tmux tests** — Watch the spinner during a streaming turn in tmux.

## Tests

### Unit tests
- Frame index advances normally.

### E2E tests
- Snapshot test of status bar during streaming.

### Live tmux tests
- Submit a prompt and observe the spinner animation.
