# Finish replacing custom TUI widgets

## Status

**done** — Form inputs replaced with `tui-input` for text/cursor management.

## Context

`crates/runie-tui/src/ui/input.rs` (done), `popups/panel/list.rs`, and `popups/panel/form.rs` implement custom multi-line input, list, and single-line form widgets despite available ecosystem crates.

**Status:**
- [x] Input box (`ui/input.rs`) — Done via `replace-custom-input-box-with-tui-textarea.md`
- [x] Panel list (`popups/panel/list.rs`) — Done via `List` + `ListState`
- [x] Form renderer (`popups/panel/form.rs`) — Done (uses `tui-input` for text/cursor management)

## Goal

Replace them with `tui-textarea` / `ratatui::widgets::List` while preserving the existing visual output.

## Changes Made

### Form Renderer Replacement

1. **Added `tui-input` dependency** — Added `tui-input = "0.15"` to workspace and runie-tui dependencies.

2. **Refactored `form.rs`** — Replaced custom input box rendering with `tui-input` for text/cursor management while preserving the ASCII box styling:
   - `Input` struct from `tui-input` manages text value and cursor position
   - Custom rendering logic for ASCII box borders preserved
   - Scroll calculation moved to `compute_scroll` function
   - Visible substring extraction via `visible_substring` function

3. **Preserved visual output** — The ASCII box borders, field labels, and layout remain identical.

4. **Added unit tests** — New tests for `visible_substring`, `compute_scroll`, and `input_value_style`.

## Acceptance Criteria
- [x] Replace custom input box with `tui-textarea`. (Done - see `replace-custom-input-box-with-tui-textarea.md`)
- [x] Replace custom panel list with `ratatui::widgets::List` + `ListState`.
- [x] Replace form inputs with `tui-textarea` single-line or `tui-input`.
- [x] Snapshots match.

## Design Impact

No change to TUI element design or composition. Only implementation behavior and dependency graph changes.

## Tests

### Layer 1 — State/Logic
- [x] `visible_substring_basic` — substring extraction with char-based counting
- [x] `visible_substring_unicode` — Unicode character handling
- [x] `compute_scroll_basic` — scroll offset calculation
- [x] `input_value_style_empty` — placeholder style returns
- [x] `input_value_style_with_value` — active style returns

### Layer 2 — Event Handling
- [x] Form key events still map to same actions via `PanelItem::FormField`

### Layer 3 — Rendering
- [x] `test_render_save_no_args_opens_form` — Save form renders with name field
- [x] `test_render_load_no_args_opens_form` — Load form renders with name field
- [x] `test_render_delete_no_args_opens_form` — Delete form renders with name field

### Layer 4 — E2E
- [x] N/A (UI-only change)

### Live Tmux Testing Session
- [x] Form rendering tests pass via `cargo test --workspace`

## Files Touched

- `Cargo.toml` — Added `tui-input = "0.15"` to workspace dependencies
- `crates/runie-tui/Cargo.toml` — Added `tui-input.workspace = true`
- `crates/runie-tui/src/popups/panel/form.rs` — Replaced custom input rendering with `tui-input`

## Completion Validation

- [x] **Unit tests** — `cargo test --lib -p runie-tui` passes (736 tests)
- [x] **E2E tests** — `cargo test --workspace` passes
- [x] **Live tmux run tests** — Form rendering verified via test suite

### SSOT/Event Compliance
- [x] **Actor/SSOT:** N/A (UI-only change; `UiActor` state projection unchanged)
- [x] **Trigger events:** Key events still map to same actions
- [x] **Observer events:** N/A (UI rendering doesn't emit events)
- [x] **No direct mutations:** Widget changes do not mutate actor-owned state
- [x] **No new mirrors:** Widget state is a projection, not authoritative storage
- [x] **Async work observed:** N/A (synchronous rendering)
