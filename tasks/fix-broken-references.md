# Fix Broken Symbol References After Login Flow Refactor

**Status**: done
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Summary

Fixed broken symbol references by archiving orphan files:

### Archived Files
- `crates/runie-core/src/update/login_flow.rs` - Module references non-existent `build_login_stack` function and is not included in the build
- `crates/runie-core/src/login_flow/tests/state.rs` - References non-existent `build_login_stack` function
- `crates/runie-core/src/login_flow/tests/integration.rs` - Archived with state.rs (sibling file, clean separation)

### Resolution
The `update/login_flow.rs` module was never included in `update/mod.rs` via `mod login_flow;`, making it dead code. The orphaned test files referenced a function (`build_login_stack`) that was never implemented.

The actual login flow logic is properly implemented inline in `update/mod.rs` using `build_login_root()`.

## Acceptance Criteria

- [x] Orphan modules/files referencing non-existent symbols archived
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` succeeds

## Notes

The actual login flow functionality is correctly implemented in `update/mod.rs`:
- `build_login_root()` - creates initial login dialog with provider picker
- `push_login_panel()`, `pop_login_panel_or_close()`, `replace_top_login_panel()` - manage panel stack
- `rebuild_login_dialog()` - rebuilds the entire login dialog
