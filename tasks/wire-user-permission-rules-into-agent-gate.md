# Wire user permission rules into the agent permission gate

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: live-tui-smoke-test-real-minimax

## Description

`RactorAgentActor::create_permission_gate` hardcodes only `DefaultToolApprove`, `GitTrackedWriteApprove`, and `FileAccessAsk`. It ignores the user's `/trust` decisions and any declarative permission rules, so custom permission configuration has no effect on tool execution.

## Root Cause

The agent builds its own `PermissionManager` from built-in defaults and does not consult `PermissionActor` or load the persisted rule set.

## Acceptance Criteria

- [ ] User `/trust` decisions affect subsequent tool approvals.
- [ ] Declarative permission rules are loaded and evaluated by the agent.
- [ ] Denials still fall back to the TUI permission dialog when no rule matches.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `native tool` scenario respects a prior `/trust bash always` decision.

## Tests

### Layer 1 — State/Logic
- [ ] `agent_gate_uses_user_trust_rules` — a configured allow-rule permits a bash call without dialog.

### Layer 2 — Event Handling
- [ ] `trust_command_updates_permission_actor` — `/trust bash always` emits a `PermissionMsg` that updates the rule set.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_trust_bash_skips_dialog` — live tmux script trusts bash, runs `native tool`, and asserts no permission dialog appears.

## Files touched

- `crates/runie-agent/src/actor.rs`
- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/commands/dsl/handlers/tool.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The permission system is already partially unified in `runie-core`; the agent just needs to use the shared rule engine.
