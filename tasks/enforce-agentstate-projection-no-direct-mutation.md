# Enforce `AgentState` as pure projection with no direct mutation

**Status**: wontfix
**Milestone**: R7
**Category**: Core / State
**Priority**: P0

**Depends on**: remove-turnstate-from-appstate
**Blocks**: none

## Summary

This task was based on an incorrect architectural assumption. The task assumed `AgentState` is a "read-only projection" of `TurnState`, but in the actual architecture, `AgentState` IS the canonical state for UI display.

### Why this task is wontfix

1. **Architectural change**: After removing `turn_state` from `AppState`, `AgentState` became the canonical state for turn/agent info, not a derived projection.

2. **Event-driven pattern is correct**: Update handlers receive events from `TurnActor` and update `AgentState` directly. This is the correct event-driven architecture:
   - Events flow: TurnActor → EventBus → Update handlers → AgentState
   - The update handlers ARE the projection layer - they transform events into state updates

3. **No dual-mutation**: With `TurnState` removed from `AppState`, there's no longer a "mirrored state" problem. `AgentState` is the only state for UI display.

4. **Direct mutation is correct**: `agent_state_mut()` calls in production code are the correct pattern - they update the canonical state based on events. This is how the MVU architecture works.

### What was done

The companion task `remove-turnstate-from-appstate.md` is now complete. It:
- Removed all references to `AppState.turn_state`
- Updated all update handlers to work with `AgentState` directly
- All tests pass

### Conclusion

The SSOT rule is now satisfied: `AgentState` is owned by `AppState` (via the `UiActor` projection), and update handlers update it directly based on events. There's no "mirrored" or "dual" state anymore.
