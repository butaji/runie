# Drop `once_cell` and `futures-util` workspace deps

**Status**: todo
**Milestone**: R7
**Category**: Dependencies
**Priority**: P2

**Depends on**: remove-unused-dependencies-and-normalize-workspace-deps
**Blocks**: none

## Description

`once_cell` is only used in `runie-tui/src/keymap.rs` and can be replaced with `std::sync::LazyLock`. `futures-util` is declared in workspace deps but all call sites use `futures::`. Remove both.

## Acceptance Criteria

- [ ] Replace `once_cell::sync::Lazy` in `keymap.rs` with `std::sync::LazyLock`.
- [ ] Remove `once_cell` from `crates/runie-tui/Cargo.toml`.
- [ ] Remove `futures-util` from workspace `Cargo.toml`.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `keymap_lazy_static_uses_std` — keymap formatter uses `LazyLock`.

## Files touched

- `Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/keymap.rs`

## Notes

- MSRV must be ≥1.80 for `LazyLock`.
