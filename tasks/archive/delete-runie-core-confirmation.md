# Delete dead `runie-core/src/confirmation.rs`

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

**N/A**: The file `crates/runie-core/src/confirmation.rs` does not exist in the current codebase. The dead code was already removed in a previous refactoring.

## Acceptance Criteria

- [ ] `crates/runie-core/src/confirmation.rs` deleted (including its `#[cfg(test)] mod tests` block).
- [ ] `pub use confirmation::{…};` removed from `crates/runie-core/src/lib.rs`.
- [ ] `rg "ConfirmationKind|ConfirmationRouter" crates/` returns zero hits outside `tasks/`.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `confirmation_module_gone` — `ls crates/runie-core/src/confirmation.rs` fails.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_workspace_builds_without_confirmation` — `cargo check --workspace` green after deletion.
- [ ] `smoke_edit_preview_flow_unchanged` — `PermissionGate::check` for edit tools still emits the same events as before.

## Files touched

- `crates/runie-core/src/confirmation.rs`
- `crates/runie-core/src/lib.rs`

## Notes

If a typed "what to confirm" enum is needed later, model it as an enum on `PermissionAction` rather than reintroducing a parallel router. The bash/diff/write distinctions already exist inside `permissions/rules.rs` and `edit_preview.rs`.
