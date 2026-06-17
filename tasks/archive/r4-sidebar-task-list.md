# Show Current Task List in Team Mode Sidebar

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: r4-orchestrator-domain-types, r4-subagent-sidebar
**Blocks**: r4-team-mode-integration

## Description

When Team mode is active, the subagent sidebar lists the Orchestrator and individual
subagents. This task adds the infrastructure for the Tasks section — `OrchestratorEvent`
types that flow through `AppState::update` into `SidebarState`, driving the task list.

## What was implemented

**Event wiring** (`crates/runie-core/src/event/`):
- `SidebarEvent` and `OrchestratorEvent` added to the top-level `Event` enum
- `Event::Orchestrator` variant with serde derives
- `to_durable()` returns `None` (transient UI state)
- Exhaustive dispatcher test covers both new variants

**Orchestrator event serde** (`crates/runie-core/src/orchestrator_actor.rs`):
- `OrchestratorEvent` derives `Serialize, Deserialize`
- `OrchestratorState` derives `Serialize, Deserialize`
- `OrchestratorCommand` derives `Serialize, Deserialize`
- `ProjectContext` derives `Serialize, Deserialize`
- `PlanStarted` event added (precedes `PlanningStarted`)

**Update handler** (`crates/runie-core/src/update/mod.rs`):
- `AppState::update` handles `Event::Orchestrator(e)` → `dispatch_event`
- `dispatch_event` handles `Event::Orchestrator(e)` → `handle_orchestrator_event`
- `handle_orchestrator_event` method:
  - `PlanStarted` → show sidebar, set orchestrator Running, clear old subagents
  - `PlanningStarted` → set orchestrator Running
  - `PlanGenerated` → populate subagent list from plan tasks
  - `SubagentStatusChanged` → update matching agent entry
  - `Cancelled` → hide sidebar, clear agents

**Integration tests** (in `crates/runie-core/src/state.rs`):
- `plan_started_shows_sidebar`
- `plan_generated_populates_subagents`
- `subagent_status_changed_updates_entry`
- `cancelled_hides_sidebar`
- `orchestrator_event_serialization` — full round-trip for all variants

## Acceptance Criteria

- [x] `OrchestratorEvent` is a variant in the top-level `Event` enum.
- [x] `OrchestratorEvent::PlanStarted/PlanGenerated/SubagentStatusChanged/Cancelled`
  drive `SidebarState` (verified by tests).
- [x] `OrchestratorEvent` serde-serializable for event bus persistence.
- [x] `cargo test --workspace` passes (1432 tests).

## Tests (Layer 1 + Layer 2)

### Layer 1 — State / Logic

- `sidebar_defaults_hidden` — default state is hidden, orchestrator focus
- `focus_defaults_to_orchestrator` — `AgentFocus::default() == Orchestrator`
- `focus_subagent_by_index` — Ctrl+1..9 selects subagent by 0-based index
- `focus_orchestrator` — Ctrl+0 returns to orchestrator focus
- `set_orchestrator_status_empty` — creates orchestrator entry on first call
- `set_orchestrator_status_updates_existing` — updates existing orchestrator entry
- `set_subagents_replaces_non_orchestrator` — subagents inserted after orchestrator
- `agent_status_serialization` — JSON round-trip for all 5 status variants
- `agent_entry_serialization` — JSON round-trip for `AgentEntry`
- `agent_focus_serialization` — JSON round-trip for both `AgentFocus` variants

### Layer 2 — Event Handling (Orchestrator → Sidebar)

- `plan_started_shows_sidebar` — `PlanStarted` makes sidebar visible, sets Running
- `plan_generated_populates_subagents` — tasks from plan become sidebar entries
- `subagent_status_changed_updates_entry` — status maps correctly, other entries unchanged
- `cancelled_hides_sidebar` — `Cancelled` hides sidebar and clears agents
- `orchestrator_event_serialization` — full round-trip for all `OrchestratorEvent` variants

## Files touched

- `crates/runie-core/src/orchestrator_actor.rs` — serde derives + `PlanStarted` event
- `crates/runie-core/src/event/mod.rs` — re-export `OrchestratorEvent`
- `crates/runie-core/src/event/variants.rs` — `Event::Orchestrator` variant + serde + to_durable + exhaustive test
- `crates/runie-core/src/update/mod.rs` — `handle_orchestrator_event` method + dispatcher
- `crates/runie-core/src/state.rs` — 5 new integration tests

## Out of scope

- Rendering the Tasks section in ratatui (sidebar widget)
- Mouse interaction with task rows
- Clicking a task to jump to its subagent feed
- Task status history or progress bars
