# Delete runie-tui/src/ipc.rs re-export shim

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/ipc.rs` is a 5-line pure re-export: `pub use runie_core::ipc::TuiIpc;`. Declared as `pub mod ipc;` in `runie-tui/src/lib.rs:21` but `rg "use runie_tui::ipc|crate::ipc::TuiIpc|runie_tui::ipc"` returns zero hits across the workspace — nobody imports from this path. Same class of dead re-export shim already flagged for `path_utils` (`delete-path-utils-reexport`) and `config_reload` (`delete-config-reload-shim`); this one was missed in the prior audit.

## Acceptance Criteria

- [ ] `crates/runie-tui/src/ipc.rs` deleted.
- [ ] `pub mod ipc;` removed from `crates/runie-tui/src/lib.rs`.
- [ ] `rg "runie_tui::ipc|crate::ipc::TuiIpc" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure re-export deletion, no logic.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_ipc_pair_still_constructs` — `runie_core::ipc::CoreIpc::new()` still builds and returns a connected `(CoreIpc, TuiIpc)` pair after the shim is gone (the canonical type lives in `runie-core`).

## Files touched

- `crates/runie-tui/src/ipc.rs` (delete)
- `crates/runie-tui/src/lib.rs` (remove `pub mod ipc;`)

## Notes

Callers (if any appear later) should import `runie_core::ipc::TuiIpc` directly. Trivial; group with `delete-path-utils-reexport` and `delete-config-reload-shim` in the same commit if convenient.
