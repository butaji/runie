# Mouse Hit-Testing & Routing

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: grok-mouse-terminal-init
**Blocks**: grok-contextual-hints

## Description

Wire crossterm mouse events into Runie's TUI. The core already had stub events
(`Event::MouseClick`, `Event::MouseRelease`, `Event::MouseDrag`, `Event::MouseMove`,
`Event::ScrollUp`, `Event::ScrollDown`) and no-crash tests. This task added
hit-testing and routing so mouse actually does something.

## What was implemented

- `MouseTarget` enum (`Feed`, `Input`, `StatusBar`, `Hints`, `Unknown`) added to `Snapshot`
- `mouse_position: Option<(u16, u16)>` added to `ViewState`
- `hovered_element: Option<usize>` added to `Snapshot` for hover hint rendering
- `input_event()` in `update/input/mod.rs` now routes:
  - `MouseScrollUp/Down` → `scroll_event()` → scroll up/down
  - `MouseClick` → hit-tests position → dispatches to input focus or feed toggle
  - `MouseMove` → updates `view.mouse_position`
- `compute_mouse_target()` and `compute_hovered_element()` are pure public functions in `snapshot.rs`
- `ui/mouse.rs` in the TUI mirrors the same math for the render actor (hover styling)

## Acceptance Criteria

- [x] Scroll wheel over the feed scrolls the conversation up/down. (`MouseScrollUp/Down` → `scroll_event`)
- [x] Left-click on a block selects it and toggles expand/collapse. (`handle_mouse_click` → `toggle_expand_all`)
- [x] Left-click in the input area focuses the prompt. (`handle_mouse_click` → exits vim nav mode)
- [ ] Middle-click pastes into the prompt (if terminal supports it). — future work
- [x] Mouse hover updates a `hovered_element` field in `Snapshot` for hint rendering. (`compute_hovered_element`)
- [x] Drag-to-select text is **out of scope** and explicitly not implemented.

## Tests

### Layer 1 — State / Logic

- `crates/runie-core/src/tests/mouse_events.rs` — existing no-crash tests pass
- `crates/runie-tui/src/ui/mouse.rs` — 9 Layer 1 pure-function tests for hit target geometry

### Layer 2 — Event Handling

- `update/input/mod.rs`: `MouseScrollUp/Down` dispatch → scroll events verified by existing scroll tests
- `handle_mouse_click` is tested implicitly via `toggle_expand_all` tests

### Layer 3 — Rendering

- `mouse.rs::compute_mouse_target` tests cover layout geometry

## Files touched

- `crates/runie-core/src/snapshot.rs` — `MouseTarget`, `compute_mouse_target`, `compute_hovered_element`
- `crates/runie-core/src/state.rs` — `mouse_position` on `ViewState`
- `crates/runie-core/src/model/cache.rs` — populates `mouse_target`, `hovered_element`, `mouse_position` in snapshot
- `crates/runie-core/src/update/input/mod.rs` — mouse event routing in `input_event` + `handle_mouse_click`
- `crates/runie-tui/src/ui/mouse.rs` — render-actor-side mouse target computation
- `crates/runie-tui/src/ui.rs` — added `mouse` module

## Out of scope

- Drag-to-select text.
- Right-click context menu.
