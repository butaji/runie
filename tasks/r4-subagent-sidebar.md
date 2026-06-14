# Subagent Sidebar

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: r4-orchestrator-actor, r4-subagent-isolation
**Blocks**: r4-team-mode-integration

## Description

Render subagents in a sidebar next to the main feed. Each agent gets a compact
status indicator, and the user can switch focus between the Orchestrator and
individual subagents with hotkeys. Show a per-agent feed when focused.

## Acceptance Criteria

- [ ] Sidebar visible only in Team mode.
- [ ] Sidebar lists Orchestrator (`Ctrl+0`) and each subagent (`Ctrl+1..9`).
- [ ] Active subagent highlighted; hotkeys change focus.
- [ ] Each agent shows status emoji/text: `pending`, `running`, `awaiting user`,
  `done`, `failed`.
- [ ] Main feed switches to show the focused agent's events.
- [ ] `Ctrl+0` always returns focus to the Orchestrator feed.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn focus_defaults_to_orchestrator() {
    let sidebar = SubagentSidebar::default();
    assert_eq!(sidebar.focus, AgentFocus::Orchestrator);
}

#[test]
fn hotkey_selects_subagent_by_index() {
    let mut sidebar = SubagentSidebar::with_agents(vec!["a".into(), "b".into()]);
    sidebar.handle(Hotkey::Subagent(2));
    assert_eq!(sidebar.focus, AgentFocus::Subagent("b".into()));
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn ctrl_zero_returns_to_orchestrator() {
    let mut app = App::with_team_mode();
    app.handle_event(ctrl_digit(1));
    app.handle_event(ctrl_digit(0));
    assert_eq!(app.sidebar.focus, AgentFocus::Orchestrator);
}
```

### Layer 3 — Rendering

```rust
#[test]
fn sidebar_renders_status_icons() {
    // TestBackend + Buffer assertion: pending, running, done icons present.
}
```

## Files touched

- `crates/runie-tui/src/sidebar.rs` (new)
- `crates/runie-tui/src/ui.rs`
- `crates/runie-tui/src/app.rs`

## Out of scope

- Mouse support in sidebar.
- Drag-and-drop reordering.
