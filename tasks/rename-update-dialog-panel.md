# Rename `update/dialog/panel.rs` to disambiguate from `dialog/panel.rs`

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Two files named `panel.rs` exist in the dialog subsystem: `dialog/panel.rs` (500 LOC — the `Panel` struct, its builder methods, `PanelView`, `PanelItem`) and `update/dialog/panel.rs` (422 LOC — the panel stack event handler, `update_panel_stack`, `PanelUpdateResult`). Both are imported in dialog-related code and the identical filename causes grep ambiguity and cognitive confusion. Rename `update/dialog/panel.rs` → `update/dialog/panel_handler.rs` (or fold into `update/dialog/mod.rs` if the handler is the module's primary content).

## Acceptance Criteria

- [ ] `update/dialog/panel.rs` renamed to `update/dialog/panel_handler.rs` (or inlined into `update/dialog/mod.rs`).
- [ ] `update/dialog/mod.rs` updated to declare the new module name.
- [ ] All `use super::panel::` or `use crate::update::dialog::panel::` imports updated.
- [ ] `arch_guardrails.rs` path strings updated if it references the file.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — pure rename, no logic change.

### Layer 2 — Event Handling
- [ ] `panel_handler_routes_events` — existing dialog panel tests in `update/dialog/panel/tests.rs` pass after rename.

### Layer 3 — Rendering
- [ ] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-core/src/update/dialog/panel.rs` → rename to `panel_handler.rs`
- `crates/runie-core/src/update/dialog/mod.rs` — update `mod panel;` → `mod panel_handler;`
- `crates/runie-core/src/update/dialog/panel/tests.rs` — update `use super::*` path if needed
- Any file importing `crate::update::dialog::panel::` — update path
- `crates/runie-core/tests/arch_guardrails.rs` — update path string if present

## Notes

Alternative: rename `dialog/panel.rs` (the builder) to `dialog/builder.rs` instead, since `update/dialog/panel.rs` (the handler) is the more "active" file. Rejected — `dialog/panel.rs` is the larger, more-referenced file and renaming it has higher blast radius. If `consolidate-login-flow-handlers` runs first, the `update/login_flow.rs` rename there sets a precedent for `_handler` suffix on handler files.
