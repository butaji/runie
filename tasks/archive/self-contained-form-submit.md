# Make Form Submit Self-Contained

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Input / Commands
**Priority**: P0

## Description

`CommandKind::Form` carries a `submit: fn() -> Event`, but the event is discarded. Actual form submission uses a brittle command-name → `Event` lookup table in `update/dialog_form.rs::build_event_for_form_command`. `FormPanel::submit_event` is stored but never used.

## Acceptance Criteria

- [ ] Forms carry their own submit logic (closure or enum) and produce the correct `Event` from field values.
- [ ] The stringly command-name lookup table is removed.
- [ ] `FormPanel::submit_event` is either used or removed.
- [ ] `CommandDef::form_panel` dead code is removed.
- [ ] All existing form commands still submit correctly.

## Tests

### Layer 1 — State/Logic
- [ ] `form_submit_produces_correct_event`.
- [ ] `form_submit_with_prefilled_args`.

### Layer 2 — Event Handling
- [ ] `slash_save_submits_run_save_command`.
- [ ] `slash_compact_submits_run_compact_command`.

### Layer 3 — Rendering
- [ ] `form_dialog_renders_fields`.
