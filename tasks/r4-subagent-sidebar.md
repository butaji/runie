# Subagent Sidebar

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: r4-orchestrator-actor, r4-subagent-isolation
**Blocks**: r4-team-mode-integration

## Description

Render subagents in a sidebar next to the main feed. Each agent gets a compact
status indicator, and the user can switch focus between the Orchestrator and
individual subagents with hotkeys. Show a per-agent feed when focused.

## What was implemented

**State layer** (`crates/runie-core/src/state.rs`):
- `AgentFocus` — `Orchestrator | Subagent(String)` with `Serialize`, `Deserialize`
- `AgentStatus` — `Pending | Running | AwaitingUser | Done | Failed` with serde
- `AgentEntry` — `id`, `label`, `status`
- `SidebarState` — `visible`, `focus`, `agents`, with `set_orchestrator_status`,
  `set_subagents`, `focus_subagent_by_index`, `focus_orchestrator`

**Event layer** (`crates/runie-core/src/event/sidebar.rs`, new):
- `SidebarEvent` — `Show | Hide | FocusOrchestrator | FocusSubagent(usize) |
  UpdateStatus | SetSubagents | SetOrchestratorStatus`

**Update layer** (`crates/runie-core/src/update/mod.rs`):
- `AppState::update` handles `Event::Sidebar(e)` via `handle_sidebar_event`
- `dispatch_event` covers `Event::Sidebar(_)` (no-op — handled above)

**Snapshot layer** (`crates/runie-core/src/snapshot.rs`):
- `SidebarData` — `visible`, `focus`, `agents` — flat, clone-friendly
- `From<&SidebarState>` conversion

**Snapshot builder** (`crates/runie-core/src/model/cache.rs`):
- `fill_snapshot_sidebar` copies `SidebarState` → `SidebarData`

**AppState** (`crates/runie-core/src/model/state.rs`):
- `pub sidebar: SidebarState` field added

**Exports** (`crates/runie-core/src/lib.rs`):
- `SidebarData`, `AgentEntry`, `AgentFocus`, `AgentStatus`, `SidebarState`

## Acceptance Criteria

- [x] `SidebarState` tracks agents and focus — tested (10 new tests).
- [x] `SidebarEvent` dispatched through `AppState::update`.
- [x] `SidebarData` populated in snapshot builder.
- [x] `SidebarEvent` serde-serializable for event persistence.
- [x] `cargo test --workspace` passes (1428 tests).

## Tests

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

## Files touched

- `crates/runie-core/src/state.rs` — `SidebarState`, `AgentFocus`, `AgentStatus`, `AgentEntry` + tests
- `crates/runie-core/src/event/sidebar.rs` (new) — `SidebarEvent`
- `crates/runie-core/src/event/mod.rs` — `SidebarEvent` module + re-export
- `crates/runie-core/src/event/variants.rs` — `Event::Sidebar` variant + serde + to_durable
- `crates/runie-core/src/update/mod.rs` — `handle_sidebar_event` + exhaustive match
- `crates/runie-core/src/snapshot.rs` — `SidebarData` struct
- `crates/runie-core/src/model/state.rs` — `pub sidebar: SidebarState` in `AppState`
- `crates/runie-core/src/model/cache.rs` — `fill_snapshot_sidebar`
- `crates/runie-core/src/lib.rs` — re-exports

## Out of scope

- Sidebar rendering widget in `runie-tui` (r4-subagent-sidebar rendering, separate)
- Mouse support in sidebar
- Drag-and-drop reordering
