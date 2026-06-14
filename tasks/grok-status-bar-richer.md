# Richer Status Bar

**Status**: todo
**Milestone**: R4
**Category**: TUI / Chrome
**Priority**: P1

**Depends on**: r4-team-mode-integration
**Blocks**: (none)

## Description

Keep Runie's chess-piece context-usage indicator, but add Grok-style metadata
that helps the user orient themselves: worktree label and orchestrator mode
indicator.

## Acceptance Criteria

- [ ] When inside a git worktree, status bar shows a worktree label
  (e.g., `worktree of ~/Code/GitHub/runie`).
- [ ] When in Team mode, status bar shows the Orchestrator state
  (`aligning`, `planning`, `executing`, etc.).
- [ ] Token usage indicator stays as the existing chess piece.
- [ ] No denominator added to the token display (decision from grilling).

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn status_bar_shows_worktree_label() {
    let snap = Snapshot {
        git_info: Some(GitInfo { is_worktree: true, source: "~/Code/GitHub/runie".into(), .. }),
        ..Default::default()
    };
    assert!(build_left_text(&snap).contains("worktree"));
}

#[test]
fn status_bar_shows_orchestrator_state_in_team_mode() {
    let snap = Snapshot {
        execution_mode: ExecutionMode::Team,
        orchestrator_state: OrchestratorState::Planning,
        ..Default::default()
    };
    assert!(build_left_text(&snap).contains("Planning"));
}
```

### Layer 3 — Rendering

```rust
#[test]
fn status_bar_renders_team_indicator() {
    // TestBackend assertion: left side contains "Planning" in Team mode.
}
```

## Files touched

- `crates/runie-core/src/snapshot.rs`
- `crates/runie-core/src/model/cache.rs`
- `crates/runie-tui/src/status_bar.rs`

## Out of scope

- Token budget denominator.
- Full Grok header layout redesign.
