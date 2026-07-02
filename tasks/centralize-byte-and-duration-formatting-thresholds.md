# Centralize byte and duration formatting thresholds

## Status

`done`

## Description

`tool/format.rs` uses raw literals for byte thresholds (`1000`, `1_000_000`, `1_000_000_000`) and duration thresholds (`60.0`). Centralize these as named constants.

## Implementation

Added constants in `crates/runie-core/src/tool/format.rs`:

```rust
// ─── Byte Formatting Thresholds ─────────────────────────────────────────────────

/// Threshold in bytes before applying kilobyte formatting.
const BYTES_PER_KB: u64 = 1_000;

/// Threshold in bytes before applying megabyte formatting.
const BYTES_PER_MB: u64 = 1_000_000;

/// Threshold in bytes before applying gigabyte formatting.
const BYTES_PER_GB: u64 = 1_000_000_000;

// ─── Duration Formatting Thresholds ────────────────────────────────────────────

/// Threshold in seconds before switching to minute/second formatting.
const SECONDS_PER_MINUTE: f64 = 60.0;
```

Updated `format_bytes` and `format_duration` to use these constants.

## Acceptance Criteria

- [x] **Unit tests** — Formatting output matches old behavior for representative values.
  - Tests in `crates/runie-core/src/tool/tests.rs`:
    - `format_bytes_small` (0, 567, 999)
    - `format_bytes_kb` (1000, 1_234_567)
    - `format_bytes_mb` (1_000_000, 3_456_789)
    - `format_duration_seconds` (12.3, 59.9)
    - `format_duration_minutes` (60.0, 65.0, 125.0)
- [x] **E2E tests** — Tool output display in replay is unchanged.
- [x] **Live tmux tests** — Run tools that produce byte/time output in tmux and verify formatting.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
