# Welcome / Launcher Screen

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Add a Grok-style welcome/launcher screen shown when no session is active. It
provides clear entry points and teaches the default hotkeys.

## Acceptance Criteria

- [ ] Welcome screen appears when there is no active session.
- [ ] Options: New session (`Ctrl+N`), Resume session (`Ctrl+S`), Quit
  (`Ctrl+Q`).
- [ ] Shows recent sessions if any exist.
- [ ] Shows a contextual tip at the bottom.
- [ ] Any printable key or `Tab` focuses the input prompt when a session is
  active.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn welcome_screen_shown_when_no_session() {
    let state = AppState::default();
    assert!(state.show_welcome);
}

#[test]
fn welcome_hidden_after_session_started() {
    let mut state = AppState::default();
    state.update(Event::NewSession);
    assert!(!state.show_welcome);
}
```

### Layer 3 — Rendering

```rust
#[test]
fn welcome_renders_new_resume_quit() {
    // TestBackend assertion: "New session", "Resume session", "Quit" visible.
}
```

## Files touched

- `crates/runie-core/src/model/state.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-tui/src/ui.rs` (welcome view)

## Out of scope

- ASCII art logo (can be added later).
- Worktree launch option (`Ctrl+W`).
