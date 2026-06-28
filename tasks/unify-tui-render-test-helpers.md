# Unify TUI render test helpers

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

Several TUI test modules duplicate `render_content`, `render_chat`, and buffer-to-string loops. Centralizing them into a shared test helper module makes UI tests cheaper to write and maintain.

Current duplication:

- `render_content(&mut AppState) -> String` in `onboarding_e2e.rs:24`, `login_flow_form.rs:22`, `login_flow_e2e.rs:22`, `line_scroll.rs:4`, `providers_e2e.rs:23`, `sticky_bottom.rs:4`, `vim_mode.rs:44`, `onboarding_render.rs:24`, `toggle_e2e.rs:4`
- `render_chat(&mut AppState, u16, u16) -> String` in `autoscroll_render.rs:4` and `tests/render/tool_truncation.rs:8`
- Buffer-to-string loops in `smoke.rs:162–167`, `vim_mode.rs:54–57`, `render/mod.rs:58–62`

## Acceptance Criteria

- [ ] Create a shared test helper module (e.g., `crates/runie-tui/src/tests/helpers.rs`).
- [ ] Move `render_content`, `render_chat`, and the buffer-to-string loop into the shared module.
- [ ] Update at least the most duplicated test files to import from the shared module.
- [ ] All existing TUI tests still pass.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [ ] `render_helpers_shared` — adds the shared helpers and updates at least two previously duplicated test modules, confirming identical rendered output.

## Files touched

- `crates/runie-tui/src/tests/*.rs`
- `crates/runie-tui/src/tests/helpers.rs` (new)

## Notes

- This is a safe, test-only refactor with no production impact.
- Out of scope: changing rendering logic or widget internals.
