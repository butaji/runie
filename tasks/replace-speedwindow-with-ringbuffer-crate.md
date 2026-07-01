# Replace `SpeedWindow` with a ring-buffer crate

## Status

`todo`

## Description

`SpeedWindow` in `crates/runie-core/src/actors/turn/state.rs` is a custom ring buffer for token-throughput averaging. The `ringbuffer` crate provides the same behavior with less code.

## Acceptance criteria

1. **Unit tests** — Speed/TPS values match the old implementation for representative token sequences.
2. **E2E tests** — Token stats events produce the same speed numbers in a replay turn.
3. **Live run tests** — Run a streaming turn in tmux and verify the speed display is stable and correct.

## Tests

### Unit tests
- Unit tests compare old and new TPS for representative token sequences.

### E2E tests
- A replay turn emits token-stat events with the same speed window values.

### Live run tests
- Run a streaming agent turn in tmux and observe the tokens-per-second display.
