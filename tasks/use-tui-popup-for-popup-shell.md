# Use tui-popup for popup shell

## Status

`todo`

## Context

`crates/runie-tui/src/popups.rs`, `popups/panel/`, `popups/permission.rs`, and `popups/welcome.rs` manually compute centered/anchored rectangles, background clearing, and borders.

## Goal

Use `tui-popup` for popup container/layout logic.

## Acceptance Criteria
- [ ] Add dependency.
- [ ] Replace manual rectangle/border math.
- [ ] Preserve custom styling and hotkey footers.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Popup snapshots unchanged.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** `/`, permission, welcome popups render.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (UI-only change; `UiActor` state projection unchanged).
- [ ] **Trigger events:** N/A (popup rendering is a read-only projection).
- [ ] **Observer events:** N/A (popup rendering doesn't emit events).
- [ ] **No direct mutations:** Popup layout changes must not mutate actor-owned state.
- [ ] **No new mirrors:** Popup state must not create authoritative copies.
- [ ] **Async work observed:** N/A (synchronous rendering).
