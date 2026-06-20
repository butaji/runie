# Fold `state/` into `model/state/`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Two "state" modules coexist in `runie-core`: `state/` (5 files: `agent.rs`, `input.rs`, `session.rs`, `view.rs`, `mod.rs` — contains `AgentState`, `CommandUsage`, `InputState`, `SessionState`, `ViewState`) and `model/state/` (4 files: `app_state.rs`, `helpers.rs`, `ranking.rs`, `types.rs` — contains `AppState` and its helpers). `model/state/helpers.rs` already imports `crate::state::CommandUsage`, creating a cross-module dependency that signals these belong together. Merge `state/*` into `model/state/` as submodules so there is one state namespace.

## Acceptance Criteria

- [ ] `state/` directory removed from src root.
- [ ] `model/state/` contains the former `state/` files as `agent.rs`, `input.rs`, `session.rs`, `view.rs`.
- [ ] `model/state/mod.rs` re-exports `AgentState`, `SpeedWindow`, `CommandUsage`, `InputState`, `SessionState`, `ViewState`, `AppState`.
- [ ] `model/mod.rs` re-exports all state types.
- [ ] `lib.rs` `pub use` lines updated to reflect new paths (external API unchanged).
- [ ] All `crate::state::` imports rewritten to `crate::model::state::` (or `super::` within `model/`).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `command_usage_tracking_works` — CommandUsage count/last_used fields accessible after move.
- [ ] `agent_state_speed_window` — SpeedWindow rolling window calculation unchanged.
- [ ] `view_state_defaults` — ViewState::default() scroll=0, cached_auth_valid=false.

### Layer 2 — Event Handling
- [ ] `input_state_tracks_cursor` — InputEvent::Input updates input buffer + cursor_pos.
- [ ] `session_state_holds_messages` — SessionState message list preserved across events.

### Layer 3 — Rendering
- [ ] N/A — pure module reorganization; rendering tests cover indirectly.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-core/src/state/` → delete (5 files moved)
- `crates/runie-core/src/model/state/agent.rs` → new (from `state/agent.rs`)
- `crates/runie-core/src/model/state/input.rs` → new (from `state/input.rs`)
- `crates/runie-core/src/model/state/session.rs` → new (from `state/session.rs`)
- `crates/runie-core/src/model/state/view.rs` → new (from `state/view.rs`)
- `crates/runie-core/src/model/state/mod.rs` — add new submodules + re-exports
- `crates/runie-core/src/model/mod.rs` — update re-exports
- `crates/runie-core/src/lib.rs` — remove `pub mod state;`, update `pub use` lines
- All files importing `crate::state::` — update import paths

## Notes

The `model/state/helpers.rs` → `crate::state::CommandUsage` import becomes `super::CommandUsage` after the move, eliminating the cross-module dependency. Rejected alternative: keep `state/` as a separate module and rename to `runtime_state/` — rejected because the conceptual boundary between "state types" and "app state" is unclear and the existing code already treats them as one family. ~15-20 files need import path updates (grep `crate::state::`).
