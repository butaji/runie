# Replace `SpeedWindow` with a ring-buffer crate

## Status

`todo`

## Description

`SpeedWindow` in `crates/runie-core/src/actors/turn/state.rs` is a custom ring buffer for token-throughput averaging. The `ringbuffer` crate provides the same behavior with less code.

## Acceptance criteria

- `SpeedWindow` implementation is replaced by `ringbuffer` or equivalent.
- Speed/TPS values remain identical for the same input sequences.

## Tests

### Layer 1 — State/Logic
- Unit tests compare old and new TPS for representative token sequences.
