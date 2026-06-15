# Solo / Team Mode Toggle

**Status**: todo
**Milestone**: R4
**Category**: Sessions
**Priority**: P0

**Depends on**: actor-runtime-decision, event-bus-jsonl-persistence
**Blocks**: r4-team-mode-integration

## Description

Introduce the execution-mode concept into Runie: **Solo** (one agent doing
planning and execution in the main session) and **Team** (an Orchestrator plans,
spawns isolated subagents, and synthesizes results). Persist the selected mode
per session and expose a TUI toggle.

## Acceptance Criteria

- [ ] `runie-core` has an `ExecutionMode` enum with `Solo` and `Team` variants.
- [ ] `Session` stores the current `ExecutionMode` and serializes it.
- [ ] `/team` toggles mode to Team, `/solo` toggles to Solo (commands added to
  `Command` enum and dispatcher).
- [ ] Mode is shown in the status bar next to the model/provider name.
- [ ] Switching mode does not clear feed history.
- [ ] Existing tests still pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn session_defaults_to_solo() {
    let session = Session::default();
    assert_eq!(session.execution_mode, ExecutionMode::Solo);
}

#[test]
fn toggling_execution_mode_updates_session() {
    let mut session = Session::default();
    session.set_execution_mode(ExecutionMode::Team);
    assert_eq!(session.execution_mode, ExecutionMode::Team);
    session.set_execution_mode(ExecutionMode::Solo);
    assert_eq!(session.execution_mode, ExecutionMode::Solo);
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn slash_team_sets_team_mode() {
    let mut app = App::default();
    app.handle_command(Command::SetExecutionMode(ExecutionMode::Team));
    assert_eq!(app.session.execution_mode, ExecutionMode::Team);
}
```

### Layer 3 — Rendering

```rust
#[test]
fn status_bar_shows_team_mode() {
    // TestBackend + Buffer assertion: status bar contains "[Team]".
}
```

## Files touched

- `crates/runie-core/src/session.rs`
- `crates/runie-core/src/command.rs`
- `crates/runie-tui/src/app.rs`
- `crates/runie-tui/src/ui.rs`

## Out of scope

- Orchestrator behavior in Team mode (covered by later R4 tasks).
- Subagent UI sidebar.
