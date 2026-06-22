# Delete dead `runie-core/src/confirmation.rs`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/confirmation.rs` (217 LOC) defines `pub enum ConfirmationKind { None, Diff { preview }, Write { … }, Bash { command, reason } }` and `pub struct ConfirmationRouter` with `for_edit` / `for_write` / `for_bash` / `for_read_only` / `to_approve_event` / `to_reject_event`. Both are re-exported via `lib.rs:106`, but `rg 'ConfirmationKind::|ConfirmationRouter::' crates/` returns only the file itself.

The live permission/approval system is `PermissionGate` + `PermissionAction` + `EditPreview` in `crates/runie-core/src/permissions/` and the edit-preview flow in `edit_preview.rs`. `ConfirmationKind::Diff { preview }` overlaps with `EditPreview`; `ConfirmationRouter::for_edit` duplicates `PermissionGate::check` for edit-style tools. The dead module is parallel to live wiring.

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
