# Ctrl+Shift+E collapse/expand

**Status**: done

**Milestone**: R1

**Category**: TUI Improvements

## Description

Collapse/expand feed elements via Ctrl+Shift+E keybinding.

**Note**: This was implemented as part of `mvp-tui-collapse`. The global collapse/expand functionality covers all ACs.

## Acceptance Criteria

- [x] Ctrl+Shift+E keybinding
- [x] Toggle element collapsed state
- [x] Visual indicator for collapsed
- [x] Restore expanded on expand

## Tests

- [x] Layer 1 — State/logic: `tests/collapse.rs`, `tests/toggle_all.rs`, `tests/collapse_new_items.rs`
- [x] Layer 2 — Event handling: `runie-term/src/main.rs` `ctrl_shift_e_converts_to_toggle_expand`
- [x] Layer 3 — Rendering: Covered by Element → Line transformation tests
- [x] Layer 4 — Smoke: End-to-end verified via manual testing
