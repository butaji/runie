# Normalize `std::sync` locks in TUI production code

**Status**: done
**Milestone**: R7

**Note**: Verified 2026-06-29 — `theme/mod.rs` uses `parking_lot::{Mutex, RwLock}` and `terminal/caps/detect.rs` uses `parking_lot::Mutex`. No `std::sync` locks remain in these files.
**Category**: Reliability
**Priority**: P1

**Depends on**: normalize-remaining-std-mutex-to-parking_lot
**Blocks**: none

## Description

`runie-tui/src/theme/mod.rs` and `runie-tui/src/terminal/caps/detect.rs` still use `std::sync::Mutex`/`RwLock` for production globals. Convert them to `parking_lot` and remove poison-recovery code.

## Acceptance Criteria

- [ ] Replace `std::sync::Mutex`/`RwLock` in `theme/mod.rs` with `parking_lot`.
- [ ] Replace `std::sync::Mutex` in `terminal/caps/detect.rs` with `parking_lot`.
- [ ] Remove `.unwrap_or_else(|e| e.into_inner())` poison recovery.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `theme_globals_use_parking_lot` — theme access works after conversion.

## Files touched

- `crates/runie-tui/src/theme/mod.rs`
- `crates/runie-tui/src/terminal/caps/detect.rs`

## Notes

- This completes the mutex normalization started in earlier tasks.
