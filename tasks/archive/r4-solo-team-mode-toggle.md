# Solo / Team Mode Toggle

**Status**: done
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

## What was implemented

- `ExecutionMode` enum (`Solo`, `Team`) in `orchestrator.rs`
  - `status_label()` → "solo" or "team"
  - `uses_orchestrator()` → true for Team
  - `Display`, `Default`, `Serialize`, `Deserialize`
- `ConfigState.execution_mode` (not session-persisted — runtime toggle)
- `Snapshot.execution_mode` for status bar rendering
- `/team` command → sets mode to Team
- `/solo` command → sets mode to Solo
- Status bar shows `[Team]` badge when in Team mode

## Acceptance Criteria

- [x] `runie-core` has an `ExecutionMode` enum with `Solo` and `Team` variants.
- [x] `ConfigState` stores the current `ExecutionMode` (runtime, not persisted).
- [x] `/team` toggles mode to Team, `/solo` toggles to Solo (commands added to
  `CommandSpec` table in `system.rs`).
- [x] Mode is shown in the status bar next to the model/provider name.
- [x] Switching mode does not clear feed history.
- [x] Existing tests still pass.

## Tests

### Layer 1 — State / Logic

- `ExecutionMode::default()` → `Solo` (verified by Default impl)
- `/team` and `/solo` handlers tested via existing command infrastructure

### Layer 2 — Event Handling

- `handle_team` / `handle_solo` update `state.config.execution_mode` and mark dirty
- Command registry integration via `CommandSpec` table (existing dispatcher handles)

## Files touched

- `crates/runie-core/src/orchestrator.rs` — added `ExecutionMode`
- `crates/runie-core/src/state.rs` — `ConfigState.execution_mode`
- `crates/runie-core/src/snapshot.rs` — `Snapshot.execution_mode`
- `crates/runie-core/src/model/cache.rs` — populates snapshot field
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — `/team`, `/solo` commands
- `crates/runie-tui/src/status_bar.rs` — `[Team]` badge when Team mode active

## Out of scope

- Orchestrator behavior in Team mode (covered by later R4 tasks).
- Subagent UI sidebar.
