# Show Current Task List in Team Mode Sidebar

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: r4-orchestrator-domain-types, r4-subagent-sidebar
**Blocks**: r4-team-mode-integration

## Description

When Team mode is active, the subagent sidebar (`tasks/r4-subagent-sidebar.md`)
lists the Orchestrator and individual subagents. This task adds a separate
**Tasks** section below the agent list that shows the current Orchestrator plan's
`SubagentTask` items and their `TaskStatus` values.

The goal is to make the Orchestrator's progress visible at a glance: users can
see *who* is working (Agents section) and *what* is being done (Tasks section)
without leaving the main chat view.

## Acceptance Criteria

- [ ] `crates/runie-core/src/snapshot.rs` exposes `team_task_items:
  Arc<[TaskListItem]>` where `TaskListItem` contains `id: String`,
  `label: String`, `status: TaskStatus`.
- [ ] `TaskStatus` is rendered with a consistent status icon:
  - `Pending` → `⏳`
  - `Running` → `▶`
  - `AwaitingUser` → `👤`
  - `Done` → `✅`
  - `Failed` → `❌`
- [ ] The Team mode sidebar renders two visually separated sections:
  - **Agents**: Orchestrator (`Ctrl+0`) + subagents (`Ctrl+1..9`) with focus
    highlight.
  - **Tasks**: current plan task list with status icons.
- [ ] A muted separator line (using the theme's separator style) appears between
  the Agents and Tasks sections.
- [ ] Task labels are truncated with ellipsis when the sidebar width is narrow.
- [ ] The Tasks section is only visible when an Orchestrator plan is active and
  Team mode is enabled.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State / Logic
- [ ] `snapshot_includes_team_task_items` — given an `AppState` with an active
  Orchestrator plan, the produced `Snapshot` contains the expected task IDs,
  labels, and statuses.
- [ ] `team_task_items_empty_in_solo_mode` — in Solo mode, `team_task_items` is
  empty even if a plan exists.
- [ ] `task_status_icon_mapping` — each `TaskStatus` variant maps to the correct
  icon character.

### Layer 2 — Event Handling
- [ ] `task_status_event_updates_sidebar_state` — publishing a task status
  event transitions the matching task in `AppState` and the next snapshot
  reflects the new status.
- [ ] `plan_started_event_populates_task_list` — an `OrchestratorPlanStarted`
  event fills `team_task_items`; an `OrchestratorPlanCleared` event empties it.

### Layer 3 — Rendering
- [ ] `sidebar_renders_agents_and_tasks_sections` — `TestBackend` + `Buffer`
  assertion shows the Agents header/rows, a separator, and the Tasks
  header/rows.
- [ ] `task_status_icons_render` — each status icon appears next to its task
  label in the rendered buffer.
- [ ] `task_labels_truncated_when_narrow` — a long task label is rendered with
  `…` when the sidebar is too narrow.

### Layer 4 — Smoke
- [ ] Extend `scripts/smoke-team-mode.sh` (from `r4-team-mode-integration`) to:
  - Switch to Team mode, submit a task, wait for the Orchestrator plan.
  - Assert the sidebar contains a "Tasks" section and at least one status icon.
  - Assert no panics, no stuck timers, and no duplicate `TurnComplete` events.

## Notes

**Why a separate section:**
- Agents and tasks are different mental models. Mixing them in one list would
  make focus switching (`Ctrl+0..9`) ambiguous and the list harder to scan.
- A dedicated Tasks section mirrors project-management UIs and gives users a
  clear progress indicator.

**Data flow:**
- `runie-core` owns `TaskListItem` and the snapshot field; no ratatui types in
  core.
- `runie-tui` maps `TaskStatus` to icons and renders the sidebar sections.
- Status updates travel via the event bus (or the existing update path if R3
  actor wiring is not yet complete).

**Status icons:**
- These are initial defaults. The theme system can later expose
  `task_status_*` glyph mappings if the design evolves.

**Files touched:**
- `crates/runie-core/src/snapshot.rs`
- `crates/runie-core/src/model/state.rs` or `orchestrator.rs`
- `crates/runie-tui/src/sidebar.rs`
- `crates/runie-tui/src/ui.rs` or `layout.rs`
- `crates/runie-core/src/event/agent.rs` or `control.rs`
- `scripts/smoke-team-mode.sh`

**Out of scope:**
- Mouse interaction with task rows.
- Clicking a task to jump to its subagent feed.
- Task status history or progress bars.
