# Name TUI and CLI channel capacities

## Status

`todo`

## Description

`runie-tui/src/ui_actor/mod.rs:216` hardcodes `16` for the effect forwarder channel. `runie-cli/src/inspect/mod.rs:579` hardcodes `16` for the inspect command's `EventBus`.

## Acceptance criteria

1. **Unit tests** — Both capacities are named constants.
2. **E2E tests** — TUI effects and inspect command still work.
3. **Live tmux tests** — Run a turn with effects and run `runie inspect` in tmux.

## Tests

### Unit tests
- Constants are used where expected.

### E2E tests
- Effect forwarder and inspect command smoke tests.

### Live tmux tests
- Submit a prompt in tmux and run the inspect CLI.
