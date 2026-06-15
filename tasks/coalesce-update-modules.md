# Coalesce Update Modules

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: flatten-event-system, complete-appstate-refactor
**Blocks**: (none)

## Description

`crates/runie-core/src/update/` contains 27 modules totaling ~4,873 lines.
Dialog handling alone spans `dialog.rs`, `dialog_form.rs`,
`dialog_panel.rs`, and `dialog_toggle.rs`. Input handling spans
`input_dispatch.rs`, `input_nav.rs`, `input_scroll.rs`, `input_text.rs`,
`input_text_support.rs`, `input_history.rs`, and `line_nav.rs`.

This fragmentation scatters related logic and forces contributors to guess
which module owns a feature.

## Acceptance Criteria

- [ ] Update modules are grouped by domain into a small number of files:
  - `input.rs` — text input, navigation, history, scrolling
  - `dialog.rs` — all dialog/palette/selector logic
  - `agent.rs` — agent lifecycle and streaming (may keep existing file)
  - `session.rs` — session commands, tree, save/load
  - `system.rs` — transient messages, diagnostics, control events
- [ ] Dead or stub functions (`is_login_flow_event_input`,
  `is_providers_event_input`, `handle_vim_nav_event_input`) are removed or
  implemented.
- [ ] `update/mod.rs` is reduced to module re-exports and the main
  `update()` dispatcher.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `update_module_count_reduced` — `ls crates/runie-core/src/update/*.rs`
  returns no more than 8 files.

### Layer 2 — Event Handling
- [ ] `all_input_events_dispatch` — each `InputEvent` variant reaches the
  correct handler after coalescing.
- [ ] `all_dialog_events_dispatch` — each `DialogEvent` variant reaches the
  correct handler after coalescing.

## Files touched

- `crates/runie-core/src/update/*.rs`

## Notes

Combine with `flatten-event-system` so the dispatcher shape is stable before
modules are merged.
