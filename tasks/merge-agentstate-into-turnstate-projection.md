# Merge AgentState into a TurnState projection

## Status

`todo`

## Context

`crates/runie-core/src/model/state/agent.rs:7-143` and `crates/runie-core/src/actors/turn/state.rs:15-114` define near-identical fields (`SpeedWindow`, queues, token counts, streaming flags). `RactorTurnActor` owns the authoritative `TurnState`, but the UI reads from `AppState::AgentState` while the actor writes to `TurnState`. The duplication survived earlier partial cleanups: `crates/runie-core/src/model/state/turn_projections.rs:199-228`, `:266-281`, `:304-325`, and `crates/runie-core/src/update/system.rs:181-212` still mutate `AgentState` directly, and `AppState::queue_steering_and_update_history` mutates `agent_state_mut().message_queue` when actor handles are absent.

## Goal

Make `AgentState` a thin read-only projection of `TurnState` (or delete it and have `AppState` hold `TurnState` directly). The `RactorTurnHandle` returns snapshots. Route all queue/state mutations through `TurnActor`.

## Acceptance Criteria

- [ ] Remove duplicated fields from `AgentState`.
- [ ] `AgentState` derives from `TurnState` snapshots/facts.
- [ ] Delete accessor glue that keeps the two copies in sync.
- [ ] UI behavior unchanged.

## Design Impact

No change to TUI element design or composition. Only internal turn-state ownership changes.

## Tests

- **Layer 1 — State/Logic:** `TurnState` transitions produce the same projection values.
- **Layer 2 — Event Handling:** `TurnActor` facts drive `AgentState` updates.
- **Layer 3 — Rendering:** `TestBackend` status/messages unchanged.
- **Layer 4 — E2E:** Provider replay fixture with multi-tool turn passes.
- **Live tmux validation:** Start a turn with streaming and tool calls; status bar and messages update correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
