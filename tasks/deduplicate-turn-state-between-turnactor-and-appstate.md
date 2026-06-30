# Deduplicate turn state between TurnActor and AppState

## Status

`todo`

## Context

`crates/runie-core/src/actors/turn/state.rs` defines `TurnState` / `SpeedWindow`. `crates/runie-core/src/model/state/agent.rs` defines `AgentState` / `SpeedWindow` with identical fields and logic. `crates/runie-core/src/update/dispatch.rs` forwards every agent event to both `TurnActor` and `AppState::AgentState`, so two parallel state machines must be kept in sync.

## Goal

Make `TurnActor` the single source of truth for turn lifecycle state. The UI projection should keep only derived read-only fields needed for rendering, received via facts (`TurnStarted`, `TokenStatsUpdated`, `TurnComplete`).

**Design impact:** No change to TUI element design or composition. Only the internal state-ownership and event-delivery behavior changes.

## Acceptance Criteria

- [ ] Extract one shared `SpeedWindow` type and delete the duplicate.
- [ ] Remove `AgentState` turn fields that mirror `TurnState`; keep only UI-derived fields.
- [ ] Route all turn state changes through `TurnActor` and apply results as facts.
- [ ] `dispatch.rs` no longer writes to `AgentState` turn fields.

## Tests

- **Layer 1 — State/Logic:** `TurnState` transitions through streaming, tool calls, and completion match expected token/queue state.
- **Layer 1:** UI projection derived from facts matches the previous duplicated `AgentState` output.
- **Layer 2 — Event Handling:** Feed `AgentEvent` deltas to `TurnActor` and assert the emitted facts drive the UI state.
- **Layer 3 — Rendering:** `TestBackend` shows correct `Working...` / token count / turn-complete state.
- **Layer 4 — E2E:** Provider replay fixture runs a multi-tool turn end-to-end and produces exactly one `TurnComplete`.
- **Live tmux validation:** Start a turn, let it stream tokens and call tools; status bar and message list update correctly and `TurnComplete` clears the working indicator.
