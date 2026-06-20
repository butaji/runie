# Inline runie-tui ipc.rs re-export shim

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/ipc.rs` is a 5-line re-export: `pub use runie_core::ipc::TuiIpc;`. The doc comment justifies keeping both endpoints in `runie-core` (to avoid a circular dep), which is correct for the *core* side, but does not justify why the TUI side needs a `crate::ipc::` alias at all. TUI callers can `use runie_core::ipc::TuiIpc;` directly and skip the indirection. Same pattern as `inline-or-document-core-ui-shim`.

## Acceptance Criteria

- [ ] All `crate::ipc::` / `runie_tui::ipc::` callers rewritten to `use runie_core::ipc::TuiIpc;` directly.
- [ ] `crates/runie-tui/src/ipc.rs` deleted.
- [ ] `pub mod ipc;` removed from `crates/runie-tui/src/lib.rs`.
- [ ] `rg "crate::ipc::|runie_tui::ipc::" crates/runie-tui/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure re-export removal.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tui_still_imports_tui_ipc` — `cargo check -p runie-tui` succeeds; the TUI binary still constructs `TuiIpc` from core.

## Files touched

- `crates/runie-tui/src/ipc.rs` (deleted)
- `crates/runie-tui/src/lib.rs` (drop `pub mod ipc;`)
- TUI callers of `crate::ipc::TuiIpc` (grep-driven)

## Notes

If a future TUI-side IPC helper (e.g. a wrapper that holds terminal caps alongside the queue) is needed, reintroduce a real `ipc` module with behaviour, not a one-line re-export. Do not keep shims "for symmetry".
