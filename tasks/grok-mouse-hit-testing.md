# Mouse Hit-Testing & Routing

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P0

**Depends on**: grok-mouse-terminal-init
**Blocks**: grok-contextual-hints

## Description

Wire crossterm mouse events into Runie's TUI. The core already has stub events
(`Event::MouseClick`, `Event::MouseRelease`, `Event::MouseDrag`, `Event::MouseMove`,
`Event::ScrollUp`, `Event::ScrollDown`) and no-crash tests. This task adds
hit-testing and routing so mouse actually does something.

## Acceptance Criteria

- [ ] Scroll wheel over the feed scrolls the conversation up/down.
- [ ] Left-click on a block selects it and toggles expand/collapse.
- [ ] Left-click in the input area focuses the prompt.
- [ ] Middle-click pastes into the prompt (if terminal supports it).
- [ ] Mouse hover updates a `hovered_element` field in `Snapshot` for hint
  rendering.
- [ ] Drag-to-select text is **out of scope** and explicitly not implemented.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn scroll_down_over_feed_increments_scroll() {
    let mut state = AppState::with_some_messages();
    let old_scroll = state.view.scroll;
    state.update(Event::ScrollDown);
    assert!(state.view.scroll > old_scroll);
}

#[test]
fn click_on_input_focuses_prompt() {
    let mut state = AppState::default();
    state.update(Event::MouseClick { row: INPUT_ROW, col: INPUT_COL, button: "left".into() });
    assert!(!state.vim_nav_mode);
    assert!(state.input_focused());
}

#[test]
fn click_on_block_toggles_expand() {
    let mut state = AppState::with_collapsible_thought();
    state.update(Event::MouseClick { row: BLOCK_ROW, col: BLOCK_COL, button: "left".into() });
    assert!(state.view.all_collapsed);
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn mouse_event_in_scrollback_emits_scroll() {
    // crossterm MouseEvent::ScrollDown at feed coords -> Event::ScrollDown
}
```

### Layer 3 — Rendering

```rust
#[test]
fn hovered_block_gets_highlight_background() {
    // TestBackend assertion: hovered post row has bg_hover style.
}
```

## Files touched

- `crates/runie-term/src/tui/events.rs` or event translation layer
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/model/cache.rs` (add hovered element to snapshot)
- `crates/runie-tui/src/ui/messages.rs` (hover style)

## Out of scope

- Drag-to-select text.
- Right-click context menu.
