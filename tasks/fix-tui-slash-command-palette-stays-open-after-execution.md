# Fix TUI slash command palette stays open after execution

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Typing a slash command such as `/session`, `/sessions`, `/copy`, or `/history` and pressing Enter executes the command, but the command palette remains open and overlays the result in the main area. This makes it hard to read command output and suggests the command is still being selected.

## Live Evidence

```
  Session: unnamed
  Messages╭ Commands ────────────────────────────────────────────────╮
  Tokens: │                                                          │
  Provider│ ❯                                                        │
  Model: e│ ──────────────────────────────────────────────────────── │
  Prompt: │   System                                               ▐ │
  ...     │ ▸ approve Apply pending file edits                       │
```

The palette covers the `/session` result.

## Acceptance Criteria

- [ ] After a slash command executes, the command palette closes automatically.
- [ ] The command result is visible in the main area without an overlay.
- [ ] The input box returns to the idle prompt.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `/session`, `/sessions`, and `/history` scenarios show the result unobscured.

## Tests

### Layer 2 — Event Handling
- [ ] `slash_command_execution_closes_palette` — simulate typing `/reset` and Enter, assert `DialogState` returns to `None`.
- [ ] `palette_open_does_not_swallow_command_result` — assert the result message is added to the chat log.

### Layer 3 — Rendering
- [ ] `slash_result_renders_without_palette_overlay` — `TestBackend` asserts no palette widget is rendered after command execution.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_slash_session_shows_result` — live tmux script runs `/session` and asserts the captured pane contains `Session:` and no `Commands` dialog border.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/update/command.rs`
- `crates/runie-core/src/update/dialog/router.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The palette is opened automatically when `/` is typed. It should close when the command is dispatched, before the result is processed.
- This affects every slash command; fixing it improves the perceived reliability of `/save`, `/load`, `/sessions`, `/history`, `/copy`, etc.
