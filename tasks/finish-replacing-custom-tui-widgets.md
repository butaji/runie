# Finish replacing custom TUI widgets

## Status

`partial`

## Context

`crates/runie-tui/src/ui/input.rs` (done), `popups/panel/list.rs`, and `popups/panel/form.rs` implement custom multi-line input, list, and single-line form widgets despite available ecosystem crates.

**Status:**
- [x] Input box (`ui/input.rs`) — Done via `replace-custom-input-box-with-tui-textarea.md`
- [x] Panel list (`popups/panel/list.rs`) — Done via `List` + `ListState`
- [ ] Form renderer (`popups/panel/form.rs`) — Remaining (supersedes `replace-custom-form-rendering-with-tui-textarea.md`)

## Goal

Replace them with `tui-textarea` / `ratatui::widgets::List` while preserving the existing visual output.

## Acceptance Criteria
- [x] Replace custom input box with `tui-textarea`. (Done - see `replace-custom-input-box-with-tui-textarea.md`)
- [x] Replace custom panel list with `ratatui::widgets::List` + `ListState`.
- [ ] Replace form inputs with `tui-textarea` single-line or `tui-input`.
- [ ] Snapshots match.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** Key events still map to same actions.
- **Layer 3 — Rendering:** `TestBackend` snapshots for input, panel, and form unchanged.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** Input, command palette, settings, save/load forms behave identically.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (UI-only change; `UiActor` state projection unchanged).
- [ ] **Trigger events:** Key events still map to same actions.
- [ ] **Observer events:** N/A (UI rendering doesn't emit events).
- [ ] **No direct mutations:** Widget changes must not mutate actor-owned state directly.
- [ ] **No new mirrors:** Widget state must be a projection, not authoritative storage.
- [ ] **Async work observed:** N/A (synchronous rendering).
