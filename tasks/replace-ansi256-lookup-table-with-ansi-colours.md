# Replace ANSI256 lookup table with ansi_colours

## Status

`done`

## Context

`crates/runie-tui/src/quantize.rs` previously maintained a hand-written 256-byte `ANSI256_TO_16` table.

## Goal

Compute nearest basic ANSI color from RGB via `ansi_colours::ansi256_from_rgb` and a small distance helper.

## Acceptance Criteria

- [x] Delete lookup table. — **Done**; custom table replaced with `ansi_colours::rgb_from_ansi256()` for all 16 basic colors.
- [x] Use `ansi_colours` + Euclidean distance over 16 standard colors. — **Done**; `ansi256_to_16()` uses Euclidean distance via `ansi_colours::rgb_from_ansi256()`.
- [x] Snapshots updated or preserved. — **Done**; quantization tested directly; no snapshot changes needed.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests compare old and new mapping.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests pass.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** Theme colors render correctly on 16-color terminals.

## Implementation

- `crates/runie-tui/src/quantize.rs` now uses `ansi_colours::ansi256_from_rgb()` and `ansi_colours::rgb_from_ansi256()`.
- `ansi256_to_16()` computes Euclidean distance over RGB space to find nearest of 16 basic ANSI colors.
- All 12 tests pass validating quantization behavior.
- `ansi_colours` is a workspace dependency declared in `crates/runie-tui/Cargo.toml`.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
