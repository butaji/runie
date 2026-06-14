# Contextual Footer Hints (Full Coverage)

**Status**: todo
**Milestone**: R4
**Category**: TUI / Chrome
**Priority**: P0

**Depends on**: grok-mouse-hit-testing, r4-solo-team-mode-toggle
**Blocks**: (none)

## Description

Runie already has state-aware hints (vim nav, `@` refs, active turn,
input active/empty). Add the remaining contexts identified from Grok:
scrollback focus, mouse hover, Team mode, and modal open.

## Acceptance Criteria

- [ ] When the feed is focused (not input, not vim nav), hints show
  `j/k scroll · enter expand · q quit`.
- [ ] When the mouse hovers a clickable block, hints show the click action
  (e.g., `click expand`).
- [ ] When Team mode is active, hints show subagent sidebar hotkeys
  (`ctrl+0 orchestrator · ctrl+1..9 agents`).
- [ ] When a modal is open (command palette, model selector, settings),
  hints show modal-specific navigation (`↑/↓ select · esc close`).
- [ ] Existing hint tests still pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn scrollback_focused_hints_show_navigation() {
    let mut state = AppState::default();
    state.focus = Focus::Feed;
    let hint = state.hint_text();
    assert!(hint.contains("j/k scroll"));
}

#[test]
fn team_mode_hints_show_subagent_hotkeys() {
    let mut state = AppState::default();
    state.session.execution_mode = ExecutionMode::Team;
    let hint = state.hint_text();
    assert!(hint.contains("ctrl+0"));
}

#[test]
fn modal_open_hints_show_close_key() {
    let mut state = AppState::default();
    state.open_dialog = Some(Dialog::CommandPalette);
    let hint = state.hint_text();
    assert!(hint.contains("esc close"));
}
```

### Layer 3 — Rendering

```rust
#[test]
fn footer_renders_team_mode_hint() {
    // TestBackend assertion: footer contains "ctrl+0" in Team mode.
}
```

## Files touched

- `crates/runie-core/src/update/input_text.rs`
- `crates/runie-core/src/update/input_text_support.rs`
- `crates/runie-core/src/model/cache.rs` (if hover focus not already in snapshot)

## Out of scope

- Custom user-defined hint strings.
