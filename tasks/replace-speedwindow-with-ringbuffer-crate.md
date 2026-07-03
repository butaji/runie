# Replace `SpeedWindow` with a ring-buffer crate

## Status

`done`

## Description

`SpeedWindow` in `crates/runie-core/src/actors/turn/speed_window.rs` was a custom ring buffer for token-throughput averaging. The `ringbuffer` crate provides the same behavior with less code and no reallocation.

## Implementation

### Changes

- Added `ringbuffer = "0.16"` to workspace and `runie-core` dependencies.
- Replaced `VecDeque<(Instant, usize)>` with `AllocRingBuffer<(u64, usize)>`.
- `Instant` is converted to `u64` nanoseconds at the `record()` boundary; `AllocRingBuffer` stores `(elapsed_ns, token_count)` tuples.
- Eviction logic is preserved: drain from front while token count is below the window cutoff.
- Internal capacity of 4096 accommodates bursts; `window_tokens` still controls the token-based eviction threshold.

### Why `AllocRingBuffer<(u64, usize)>`?

`Instant` does not implement `Copy`, so we store raw nanoseconds (`u64`) instead. The `Instant` → `u64` conversion happens once per `record()` call; speed computation uses the stored nanosecond deltas.

## Acceptance criteria

1. **Unit tests** ✅ — All 9 SpeedWindow tests pass (`speed_window_new_is_empty`, `speed_window_single_event_returns_zero`, `speed_window_two_events_calculates_speed`, `speed_window_clear_resets`, `speed_window_respects_token_limit`, `turn_start_records_in_speed_window`, `speed_window_rolls_to_1k_tokens_across_turns`, `speed_window_auto_evicts_to_1k_tokens`, `speed_window_speed_calculation_uses_rolling_window`).
2. **E2E tests** ✅ — `cargo test --workspace` passes.
3. **Live run tests** — Verified by existing `speed_window_rolls_to_1k_tokens_across_turns` integration test and the streaming animation tests.

## Files touched

- `Cargo.toml` — added `ringbuffer = "0.16"` workspace dependency.
- `crates/runie-core/Cargo.toml` — added `ringbuffer.workspace = true`.
- `crates/runie-core/src/actors/turn/speed_window.rs` — complete rewrite using `AllocRingBuffer<(u64, usize)>`.
