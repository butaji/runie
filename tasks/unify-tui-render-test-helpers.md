# Unify TUI render test helpers

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

Several TUI test modules duplicate `render_content`, `render_chat`, and buffer-to-string loops. Centralizing them into a shared test helper module makes UI tests cheaper to write and maintain.

**Completed:**

- `crates/runie-tui/src/tests/mod.rs` — added `render_content` and `render_with_size` helpers
- Updated the following test files to use shared helpers:
  - `onboarding_render.rs`
  - `onboarding_e2e.rs`
  - `login_flow_e2e.rs`
  - `login_flow_form.rs`
  - `providers_e2e.rs`
  - `toggle_e2e.rs`
  - `line_scroll.rs`
  - `sticky_bottom.rs`
  - `vim_mode.rs`

## Acceptance Criteria

- [x] Create a shared test helper module (e.g., `crates/runie-tui/src/tests/mod.rs`).
- [x] Move `render_content`, `render_chat`, and the buffer-to-string loop into the shared module.
- [x] Update at least the most duplicated test files to import from the shared module.
- [x] All existing TUI tests still pass (702 tests).
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [x] `render_helpers_shared` — shared helpers used by 9 previously duplicated test modules.

## Files touched

- `crates/runie-tui/src/tests/mod.rs` (added helpers)
- `crates/runie-tui/src/tests/onboarding_render.rs`
- `crates/runie-tui/src/tests/onboarding_e2e.rs`
- `crates/runie-tui/src/tests/login_flow_e2e.rs`
- `crates/runie-tui/src/tests/login_flow_form.rs`
- `crates/runie-tui/src/tests/providers_e2e.rs`
- `crates/runie-tui/src/tests/toggle_e2e.rs`
- `crates/runie-tui/src/tests/line_scroll.rs`
- `crates/runie-tui/src/tests/sticky_bottom.rs`
- `crates/runie-tui/src/tests/vim_mode.rs`

## Notes

- This is a safe, test-only refactor with no production impact.
- Out of scope: changing rendering logic or widget internals.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
