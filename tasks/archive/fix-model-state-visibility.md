# Fix private model state module blocking lib compilation

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

`crates/runie-core/src/model/mod.rs` declared `mod state;` (private) while re-exporting `crate::model::state::types::InputReceiver`. Several source files imported `crate::model::state::AppState`. With the private module this produced an accessibility error, preventing `cargo test -p runie-core --lib` from compiling.

## Acceptance Criteria

- [x] Change `mod state;` to `pub(crate) mod state;` in `crates/runie-core/src/model/mod.rs`.
- [x] `cargo check -p runie-core` succeeds.
- [x] `cargo test -p runie-core --lib` succeeds.

## Tests

- [x] Layer 4 Smoke: `cargo check -p runie-core` succeeds.
- [x] Layer 4 Smoke: `cargo test -p runie-core --lib` passes.

## Files touched

- `crates/runie-core/src/model/mod.rs`

## Notes

`pub(crate)` keeps the module internal to the crate while allowing the existing re-exports and internal imports to compile.
