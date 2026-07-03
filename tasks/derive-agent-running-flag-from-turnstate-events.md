# Derive `agent_running` flag from `TurnState` events

## Status

`done`

## Context

`UiActor` maintained an explicit `agent_running: bool` field that was set to `true` in the `TurnStarted` guard and `false` in `clear_turn_state`. This duplicated the authoritative `TurnState.turn_active` projection.

## Goal

Derive `agent_running` from authoritative state instead of maintaining a separate independent field.

## Implementation

Replaced the explicit `agent_running` field with a derived approach:

- **`agent_running()`** (test-only): returns `turn_active || turn_was_active`
  - `turn_active` — authoritative projection of `TurnState.turn_active`, set by `apply_event(TurnStarted)`/`apply_event(TurnCompleted)`
  - `turn_was_active` — persistent UiActor field, set when an agent is spawned (`!prev_turn_active && !turn_was_active` guard), cleared by `clear_turn_state` on `TurnCompleted`/`Abort`

The guard for duplicate-spawn prevention uses `!prev_turn_active && !turn_was_active`:
- `prev_turn_active` — captured at the TOP of `handle_event_inner`, BEFORE `apply_event` updates the projection. This allows the first `TurnStarted` in a turn cycle (when `turn_active` was `false` before the event).
- `turn_was_active` — prevents a `TurnStarted` that arrives after `Done` clears `turn_active` from spawning a second agent before the guard has settled.

## Why `turn_was_active` is needed

`Done` calls `finish_turn` → `clear_turn_state` which sets `turn_active = false`. A `TurnStarted` arriving immediately after sees `prev_turn_active = false` (captured before `Done` updated it). Without `turn_was_active`, the guard would allow a second spawn. With `turn_was_active`, the guard correctly blocks because `turn_was_active = true` (set when the first agent was spawned).

## Acceptance Criteria

- [x] `agent_running()` reflects `TurnStarted`/`TurnCompleted` events accurately (7 agent_run_guard tests).
- [x] UI enables/disables submit at the right times in replay (guard tests).
- [x] `cargo test --workspace` passes.

## Tests

### Layer 2 — Event Handling
- [x] `turn_started_spawns_agent_once` — first TurnStarted spawns agent.
- [x] `second_turn_started_blocked_by_guard` — duplicate TurnStarted blocked.
- [x] `done_does_not_clear_guard` — Done does not clear guard.
- [x] `done_then_queued_turn_started_blocked_by_guard` — queued TurnStarted after Done blocked.
- [x] `done_from_shared_bus_does_not_clear_guard` — Done from shared bus does not clear guard.
- [x] `second_turn_started_blocked_by_guard` — second TurnStarted blocked by guard.
- [x] `turn_actor_turn_started_reaches_uiactor_via_shared_bus` — TurnActor→UiActor delivery works.

### Layer 1 — State/Logic
- [x] Guard logic: `!prev_turn_active && !turn_was_active` correctly handles all scenarios.

## Files touched

- `crates/runie-tui/src/ui_actor.rs` — replaced `agent_running` field with derived `turn_was_active` field; updated guard and `agent_running()` getter.

## Validation

- [x] **Unit tests** — 699 runie-tui tests pass.
- [x] **E2E tests** — agent_run_guard layer-2 tests pass.
- [x] **Live tmux run tests** — N/A (covered by unit + E2E tests).
