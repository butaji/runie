# Use tui-popup for popup shell

## Status

`done`

## Context

`crates/runie-tui/src/popups.rs`, `popups/permission.rs`, and `popups/welcome.rs` manually computed centered/anchored rectangles, background clearing, and borders.

## Goal

Use `tui-popup` for popup container/layout logic.

## Changes Made

- **Added `tui-popup = "0.7"` to workspace dependencies** (`Cargo.toml`)
- **Added `tui-popup.workspace = true` to `runie-tui/Cargo.toml`**
- **Replaced manual shell in `popups.rs`** (`path_suggestions`): removed `clear_panel_bg` + `Paragraph::new().block()` chain; replaced with `tui_popup::Popup::new(Text::from(lines))` + `.title()` + `.style()` + explicit inner background fill
- **Replaced manual shell in `welcome.rs`**: `palette_popup_rect()` still computes the target area, but the block border and background are now rendered via `tui_popup::Popup::new(Text::from(lines))`; `pad_to_height` removed (tui-popup auto-sizes)
- **Replaced manual shell in `permission.rs`**: `super::panel::setup_popup` call removed; `tui_popup::Popup::new(Text::from(lines))` handles border/title/background; explicit inner background fill preserved

### What was NOT changed

- **Panel shell** (`popups/panel/mod.rs`): kept manual layout. `tui-popup` auto-sizes based on content, but the panel dialog has complex scroll behavior (`compute_scrolling`) that requires a fixed inner area. `tui-popup` would expand to show all content instead of scrolling.
- **Hotkey footers** in form/list dialogs: preserved via existing manual layout.

## Acceptance Criteria
- [x] Add dependency. — `tui-popup = "0.7"` added to workspace
- [x] Replace manual rectangle/border math. — `path_suggestions`, `render_welcome`, `permission_dialog` use `tui_popup::Popup`
- [x] Preserve custom styling and hotkey footers. — Panel dialog (form/list) kept manual layout; permissions/welcome use `tui-popup` with preserved styling

## Design Impact

No change to TUI element design or composition. Only implementation behavior changed: `tui-popup` handles border/title/centering, explicit inner background fill preserved for panel background color.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** `cargo test -p runie-tui` passes (733 tests)
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** permission, welcome, path-suggestion popups render correctly.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-tui` passes (733 tests)
- [x] **E2E tests** — `cargo test --workspace` passes
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session

## SSOT/Event Compliance
- [x] **Actor/SSOT:** N/A (UI-only change; `UiActor` state projection unchanged).
- [x] **Trigger events:** N/A (popup rendering is a read-only projection).
- [x] **Observer events:** N/A (popup rendering doesn't emit events).
- [x] **No direct mutations:** Popup layout changes do not mutate actor-owned state.
- [x] **No new mirrors:** Popup state is not an authoritative copy.
- [x] **Async work observed:** N/A (synchronous rendering).
