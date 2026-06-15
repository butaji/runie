# Team Mode Integration

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: r4-solo-team-mode-toggle, r4-orchestrator-actor, r4-subagent-isolation, r4-subagent-sidebar, r4-sidebar-task-list
**Blocks**: (none)

## Description

Wire Solo/Team mode selection into the runtime. When Team mode is active, user
requests go to the `OrchestratorActor` instead of the main agent. Integrate the
sidebar, event bus, command dispatcher, and session persistence.

## Acceptance Criteria

- [ ] In Team mode, pressing Enter on a user message routes to
  `OrchestratorActor::StartRequest`.
- [ ] In Solo mode, behavior is unchanged from before R4.
- [ ] Orchestrator events are persisted to the event bus and replayed on
  session load.
- [ ] `/solo` while an orchestration is running cancels it and returns to Solo.
- [ ] Status bar reflects current mode and orchestrator state.
- [ ] Smoke test runs the binary in Team mode and checks for no panics.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn router_uses_orchestrator_in_team_mode() {
    let mut router = RequestRouter::new();
    router.session.set_execution_mode(ExecutionMode::Team);
    let target = router.target("hello");
    assert!(matches!(target, RequestTarget::Orchestrator(_)));
}

#[test]
fn router_uses_agent_in_solo_mode() {
    let mut router = RequestRouter::new();
    let target = router.target("hello");
    assert!(matches!(target, RequestTarget::Agent));
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn enter_in_team_mode_starts_orchestrator() {
    let mut app = App::with_team_mode();
    app.input = "refactor auth".into();
    app.handle_event(press_enter());
    assert!(app.orchestrator.is_some());
}
```

### Layer 4 — Smoke

```bash
# New smoke test: runie-team-mode-smoke.sh
# - Launches runie, sends /team, submits a task, waits for plan/synthesis,
# - checks no panic, no stuck timer, no duplicate TurnComplete.
```

## Files touched

- `crates/runie-tui/src/app.rs`
- `crates/runie-tui/src/handler.rs`
- `crates/runie-core/src/router.rs` (new)
- `scripts/smoke-team-mode.sh` (new)

## Out of scope

- General multi-session crews.
- External A2A/MCP agents in this milestone.
