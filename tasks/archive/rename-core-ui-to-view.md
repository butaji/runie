# Rename core-ui to view

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: fold-state-into-model-state
**Blocks**: generate-ui-elements-from-model, split-runie-core-into-domain-and-io-crates

## Description

Rename `core_ui` module to `view` for consistency with the architecture documentation. The `view/` module is the domain projection layer that converts facts into renderable state.

## Acceptance Criteria

- [ ] `core_ui` renamed to `view`
- [ ] All imports updated
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- N/A (renaming task)

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- [ ] `view_elements_render_correctly`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/core_ui/` → `crates/runie-core/src/view/`
- `crates/runie-tui/src/core_ui/` → `crates/runie-tui/src/view/`
- All files importing `core_ui`

## Notes

- This is a simple rename task
- Keep the functionality unchanged
