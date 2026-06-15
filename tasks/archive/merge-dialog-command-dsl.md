# Merge Command DSL Forms into Dialog DSL

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Input / Commands
**Priority**: P1

## Description

`runie-core` has two similar DSLs for panels and forms:

- `crates/runie-core/src/dialog/dsl/` — generic panel/form builder.
- `crates/runie-core/src/commands/dsl/` — command definition DSL with its own
  `FormBuilder`, `FormField`, and `CommandFlow::Form`.

The command DSL should reuse the dialog DSL so forms are defined in one place.

## Acceptance Criteria

- [x] `commands/dsl/builder.rs::form()` delegates to `dialog/dsl` builders.
- [x] `CommandFlow::Form` variant is removed; form commands use `CommandFlow::PanelStack`.
- [x] `commands/dsl/flow.rs::exec` no longer contains a separate form-execution branch.
- [x] All slash-command forms (`save`, `load`, `delete`, `export`, `import`, `compact`,
  `fork`, `name`, `prompt`) still render and submit correctly.

## Tests

### Layer 1 — State/Logic
- [x] `command_form_builds_panel_stack`.
- [x] `form_prefills_args`.

### Layer 2 — Event Handling
- [x] Existing slash save/load tests exercise form-open events.

### Layer 3 — Rendering
- [x] `form_dialog_renders_fields` covered by existing command/tests form panels.

## Files touched

- `crates/runie-core/src/commands/dsl/builder.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/commands/dsl/mod.rs`
- `crates/runie-core/src/commands/handlers/*.rs` that call `.form()`
- `crates/runie-core/src/dialog/dsl/form.rs`

## Out of scope

- Rewiring the render actor to use the dialog DSL (rendering already uses it).
