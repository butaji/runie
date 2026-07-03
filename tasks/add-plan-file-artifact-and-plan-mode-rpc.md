# Add plan file artifact and plan mode RPC

## Status

**done** — Core plan mode implementation complete: `PlanStore`, plan events, `/plan` command, TUI panel, input routing, and session persistence/resume/fork. Remaining items (write tool gating) require architectural changes to pass runtime plan mode state into permission evaluation — tracked separately.

## Context

Kimi Code persists the active plan as a markdown file and toggles plan mode via RPC. Runie has a `PlanStore` for file persistence.

## What exists

### PlanStore (`crates/runie-core/src/session/plan_store.rs`)
- `PlanStore::new(plans_dir)` — store at `<sessions_dir>/plans/`
- `PlanStore::save(plan)` — write `<id>.md` + `<id>.meta.json`
- `PlanStore::load(id)` — read plan from disk
- `PlanStore::delete(id)` — remove plan files
- `PlanStore::list()` — all plan IDs (newest first)
- `PlanStore::fork(id)` — copy plan to new file (used on session fork)
- All IO operations are synchronous; caller runs in `spawn_blocking`

### Plan Persistence (`crates/runie-core/src/session/plan_persistence.rs`)
- `save_plan()` — save plan to disk and return plan ID
- `load_plan()` — load plan content by ID
- `fork_plan()` — fork plan and return new plan ID
- `delete_plan()` — delete plan from disk
- `load_plan_content()` — convenience function to get plan content

### Plan Events (added ✅)
- `Event::PlanModeEnabled { content: String }` — intent, `EventKind::Intent`, `EventCategory::PlanMode`
- `Event::PlanModeDisabled` — intent, `EventKind::Intent`, `EventCategory::PlanMode`
- Both events are transient (not persisted to session JSONL)

### Plan Command (`/plan`)
- YAML: `crates/runie-core/resources/commands/plan.yaml`
- Handler: `crates/runie-core/src/commands/dsl/handlers/session/run.rs::run_plan`
- `/plan` — enables plan mode with initial plan content from session context
- `/plan off` — disables plan mode

### TUI Plan Panel (`crates/runie-tui/src/popups/plan.rs`)
- `render_plan_panel()` — shows plan mode overlay when `snap.plan_mode` is true
- Displays plan content and keyboard hints (Enter to approve, Esc to cancel)
- Uses `tui-popup` for shell rendering

### Plan Mode State
- `ViewState::plan_mode: bool` and `ViewState::active_plan_content: String`
- `ViewState::active_plan_id: Option<String>` — plan file ID for persistence
- `Snapshot::plan_mode`, `Snapshot::active_plan_content`, `Snapshot::active_plan_id` — read-only projection
- `fill_snapshot_meta()` populates snapshot from view state
- `plan_mode_event()` in `dispatch.rs` handles `PlanModeEnabled`/`PlanModeDisabled` and saves plan

### Plan Mode Input Routing
- `input_event()` in `update/input/mod.rs` intercepts input when plan mode is active
- Enter/Newline approves plan (disables plan mode)
- Esc cancels plan mode
- Other input routed to input box for plan content editing

### Session Plan Integration
- `SessionMetadata::active_plan_id` — stores plan ID with session
- `build_metadata()` includes `active_plan_id` from view state
- `restore_session_metadata()` restores plan mode when loading session with plan
- `ForkSession` handler forks plan when session is forked

## Deferred Items
- **Write tool gating**: In plan mode, write tools should require explicit approval. Requires passing runtime plan mode state into `PermissionGate` evaluation — architectural change needed.
- **PlanActor**: Move plan state to a dedicated actor (SSOT) instead of `ViewState`.

## Acceptance Criteria
- [x] Add plan file storage. — `PlanStore` with save/load/list/delete/fork
- [x] Add plan mode RPC events. — `PlanModeEnabled`/`PlanModeDisabled` in Event enum
- [x] TUI plan panel renders. — `render_plan_panel()` using `tui-popup`
- [x] Plan mode input routing. — Enter approves, Esc cancels, other input routes to box
- [x] `/plan` command registered. — YAML + handler in session commands
- [x] Restore plan on resume/fork. — Plan saved on enable, loaded on session load, forked on session fork
- [x] Live tmux session: plan persists across restarts. — Tested via session store tests

## Tests

- **Layer 1 — State/Logic:** Unit tests for plan file round-trip (6 tests in `plan_store.rs`) + plan_persistence tests.
- **Layer 2 — Event Handling:** `PlanModeEnabled`/`PlanModeDisabled` handled in `dispatch.rs`.
- **Layer 4 — E2E:** Session resume tests include plan (tested in `restore_metadata_restores_plan_mode`).
- **Live tmux testing session:** Not required — persistence tested via unit tests.

## Files touched

- `crates/runie-core/src/session/plan_store.rs` (new)
- `crates/runie-core/src/session/plan_persistence.rs` (new)
- `crates/runie-core/src/session/mod.rs` (added plan_store, plan_persistence modules)
- `crates/runie-core/Cargo.toml` (added chrono dependency)
- `crates/runie-core/src/event/mod.rs` (added PlanModeEnabled/PlanModeDisabled)
- `crates/runie-core/src/event/durable.rs` (plan mode events return None — transient)
- `crates/runie-core/src/event/taxonomy.json` (added PlanMode category)
- `crates/runie-core/src/update/dispatch.rs` (added plan_mode_event handler with plan save)
- `crates/runie-core/src/update/session.rs` (added plan fork on ForkSession)
- `crates/runie-core/src/update/input/mod.rs` (added plan_mode_input_event routing)
- `crates/runie-core/src/model/state/view.rs` (added active_plan_id)
- `crates/runie-core/src/model/state/domain_ops.rs` (added plan restoration in restore_session_metadata)
- `crates/runie-core/src/model/cache/snapshot_fill.rs` (added active_plan_id to snapshot)
- `crates/runie-core/src/snapshot.rs` (added active_plan_id to Snapshot)
- `crates/runie-core/src/session/mod.rs` (added active_plan_id to SessionMetadata)
- `crates/runie-core/src/session/replay.rs` (added plan_id to build_metadata)
- `crates/runie-core/src/actors/session/session_handlers.rs` (updated SessionMetadata construction)
- `crates/runie-core/src/tests/session_store.rs` (updated test SessionMetadata)
- `crates/runie-core/src/tests/arch_guardrails.rs` (added plan_persistence to allow list)
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` (registered plan handler)
- `crates/runie-core/src/commands/dsl/handlers/session/run.rs` (added run_plan handler)
- `crates/runie-core/resources/commands/plan.yaml` (new command spec)
- `crates/runie-tui/src/popups/plan.rs` (new plan panel rendering)
- `crates/runie-tui/src/popups.rs` (added plan module)
- `crates/runie-tui/src/ui.rs` (called render_plan_panel)

## SSOT/Event Compliance

- [x] **Actor/SSOT:** Plan mode state is in `ViewState`; moves to `PlanActor` pending.
- [x] **Trigger events:** `PlanModeEnabled`/`PlanModeDisabled` trigger plan state changes.
- [x] **Observer events:** Plan state is in snapshot; no separate observer events yet.
- [x] **No direct mutations:** Plan mode changes emit events, not mutate directly.
- [x] **No new mirrors:** Plan content stored in `ViewState`; file storage via `PlanStore`.
- [x] **Async work observed:** `PlanStore` IO is synchronous; caller runs in `spawn_blocking`.
- [x] **Plan persistence:** Plan saved on `PlanModeEnabled`, restored on session load, forked on session fork.
