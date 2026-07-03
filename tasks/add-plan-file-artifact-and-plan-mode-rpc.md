# Add plan file artifact and plan mode RPC

## Status

**partial** — `PlanStore` implemented with full round-trip persistence; plan events added to Event enum; plan commands registered; fork copy wired. Remaining: live tmux testing, plan mode blocking behavior (write tool gating), and TUI integration.

## Context

Kimi Code persists the active plan as a markdown file and toggles plan mode via RPC. Runie has no plan artifact.

## Goal

Persist plans as `<session_dir>/plans/<id>.md`, emit `PlanModeEnabled`/plan-file facts, and copy the plan on fork.

## Implementation

### Phase 1: PlanStore (done ✅)

`crates/runie-core/src/session/plan_store.rs` implements:
- `PlanStore::new(plans_dir)` — store at `<sessions_dir>/plans/`
- `PlanStore::save(plan)` — write `<id>.md` + `<id>.meta.json`
- `PlanStore::load(id)` — read plan from disk
- `PlanStore::delete(id)` — remove plan files
- `PlanStore::list()` — all plan IDs (newest first)
- `PlanStore::fork(id)` — copy plan to new file (used on session fork)
- All IO operations are synchronous; caller runs in `spawn_blocking`

### Phase 2: Plan Events (done ✅)

Added to `Event` enum:
- `PlanModeEnabled { plan_id, content }` — intent
- `PlanModeDisabled` — intent
- `PlanSaved { plan_id }` — fact
- `PlanLoaded { plan_id, content }` — fact
- `PlanForked { old_plan_id, new_plan_id }` — fact

### Phase 3: Plan Commands (done ✅)

Slash commands registered:
- `/plan` — enable plan mode (creates new plan)
- `/plan save` — save current plan
- `/plan list` — list saved plans
- `/plan load <id>` — load a plan
- `/plan delete <id>` — delete a plan
- `/plan off` — disable plan mode

### Phase 4: Fork Copy (done ✅)

`run_fork` command copies the active plan to the forked session.

### Remaining Work

- **Live tmux testing**: Plan mode persists across restarts.
- **TUI integration**: Plan panel renders current plan content.
- **Write tool gating**: In plan mode, write tools require explicit approval.
- **PlanActor**: Move plan state to a dedicated actor (SSOT).

## Acceptance Criteria

- [x] Add plan file storage. — `PlanStore` with save/load/list/delete/fork
- [x] Add plan mode RPC events. — PlanModeEnabled/Disabled/Saved/Loaded/Forked
- [ ] Restore plan on resume/fork. — Fork copy wired; resume loading pending
- [ ] Live tmux session: plan persists across restarts.

## Tests

- **Layer 1 — State/Logic:** Unit tests for plan file round-trip (6 tests).
  - `plan_store_round_trip` — save + load preserves content
  - `plan_store_list` — lists saved plans
  - `plan_store_fork` — fork creates new plan with same content
  - `plan_store_delete` — delete removes plan
  - `plan_store_nonexistent_returns_none`
- **Layer 4 — E2E:** Session resume tests include plan (pending)
- **Live tmux testing session (required):** Plan mode persists across restarts (pending)

## Files touched

- `crates/runie-core/src/session/plan_store.rs` (new)
- `crates/runie-core/src/session/mod.rs` (added plan_store module)
- `crates/runie-core/Cargo.toml` (added chrono dependency)
- `crates/runie-core/src/event/mod.rs` (added PlanMode events)
- `crates/runie-core/src/commands/dsl/handlers/` (plan commands)

## SSOT/Event Compliance

- [ ] **Actor/SSOT:** `SessionActor` or `PlanActor` owns plan state. — Pending: PlanActor not yet created
- [x] **Trigger events:** `PlanModeEnabled`, `PlanSaved`, `PlanForked` trigger plan operations.
- [x] **Observer events:** `PlanModeEnabled`, `PlanLoaded` notify observers.
- [x] **No direct mutations:** Plan changes emit events, not mutate state directly.
- [x] **No new mirrors:** Plan file is authoritative; in-memory projection rebuilt from events.
- [x] **Async work observed:** Plan file IO is synchronous; caller runs in `spawn_blocking`.
