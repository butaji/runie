# Fix initial TUI snapshot race after bootstrap

## Status

`todo`

## Description

The first TUI snapshot may be sent before `EnvDetected`/`ConfigLoaded` arrive, so the first frame lacks `cwd_name`/`git_info`. Ensure the initial snapshot is published after bootstrap facts are applied.

## Acceptance criteria

1. **Unit tests** — Initial snapshot includes cwd/git after bootstrap events.
2. **E2E tests** — TUI bootstrap sequence produces a correct first frame.
3. **Live tmux tests** — Launch tmux and verify the status bar shows cwd/git immediately.

## Tests

### Unit tests
- Bootstrap event ordering.

### E2E tests
- First rendered frame contains expected env info.

### Live tmux tests
- Start the app and check status bar.
