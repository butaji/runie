# Drop `once_cell` and `futures-util` workspace deps

**Status**: done
**Milestone**: R7
**Category**: Dependencies
**Priority**: P2

**Depends on**: remove-unused-dependencies-and-normalize-workspace-deps
**Blocks**: none

## Description

`once_cell` was already replaced with `std::sync::LazyLock` in `keymap.rs`. `futures-util` is not declared as a direct dependency in any crate or workspace Cargo.toml — all `futures` usage comes transitively from other deps. No changes were needed.

## Acceptance Criteria

- [x] Replace `once_cell::sync::Lazy` in `keymap.rs` with `std::sync::LazyLock`. (Already done.)
- [x] Remove `once_cell` from `crates/runie-tui/Cargo.toml`. (Not declared.)
- [x] Remove `futures-util` from workspace `Cargo.toml`. (Not declared.)
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `keymap_lazy_static_uses_std` — verified: `keymap.rs` uses `std::sync::LazyLock`.

## Files touched

None — task was already satisfied by earlier refactors.

## Notes

- `keymap.rs` line 7: `use std::sync::LazyLock;` — already using the std version.
- `futures-util` was never a direct workspace dependency; `futures` itself is used via the `futures` crate.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
