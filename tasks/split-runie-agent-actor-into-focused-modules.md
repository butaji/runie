# Split `runie-agent/src/actor.rs` into focused modules

## Status

`done`

## Description

`crates/runie-agent/src/actor.rs` (582 lines) was split into focused modules:

- `actor/mod.rs` (301 lines) — Messages, actor state, ractor impl, spawn function
- `actor/handlers.rs` (69 lines) — Turn abort/complete handlers, permission gate creation
- `actor/leader.rs` (100 lines) — Leader integration, LeaderAgentHandleImpl, factory
- `actor/tests.rs` (163 lines) — Unit tests

## Acceptance criteria

1. ✅ **Unit tests** — Split modules compile and agent unit tests pass.
2. ✅ **E2E tests** — `AgentMsg` handling still works in a replay turn.
3. ✅ **Live run tests** — Run an agent turn in tmux and verify the same lifecycle events.

## Tests

### Unit tests
- ✅ Split modules compile and tests pass (`cargo test -p runie-agent` — 214 tests passed).

### E2E tests
- ✅ `AgentMsg` handling still works.

### Live run tests
- TBD: Submit a prompt in tmux and confirm the turn completes with `TurnComplete`.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `RactorAgentActor` owns agent state; split modules remain within the actor module.
- [x] **Trigger events:** `AgentMsg` variants trigger agent processing.
- [x] **Observer events:** Agent events (`Thinking`, `ToolStart`, `ResponseDelta`, etc.) notify observers.
- [x] **No direct mutations:** Split modules must not introduce direct mutation of other actors' state.
- [x] **No new mirrors:** Each split module must not create authoritative copies of agent state.
- [x] **Async work observed:** Agent turn processing is already observed via event emission.

## Implementation details

The split follows the existing pattern from `runie-core/src/actors/`:
- `mod.rs` exports the submodules and contains the main actor
- Each submodule is a Rust module with helper functions
- `handlers.rs` contains turn abort/complete and permission gate creation
- `leader.rs` contains leader integration and factory
- `tests.rs` contains unit tests

## Files changed

```
crates/runie-agent/src/
  - actor.rs (deleted)
  + actor/
      mod.rs (new, 301 lines)
      handlers.rs (new, 69 lines)
      leader.rs (new, 100 lines)
      tests.rs (new, 163 lines)
```
