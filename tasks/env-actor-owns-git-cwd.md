# EnvActor owns git info and cwd name

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry
**Blocks**: none

## Description

`git_info` and `cwd_name` are set once during TUI bootstrap. The architecture should use an actor to detect them asynchronously and emit an event.

## Implementation Summary

### Completed Work (2026-06-25)

- ✅ `IoActor` has `DetectEnv` message and `detect_env()` implementation
- ✅ `IoActor` emits `Event::EnvDetected { cwd_name, git_info }`
- ✅ `runie-tui/src/app_init.rs` sends `IoMsg::DetectEnv` instead of direct assignment
- ✅ `dispatch.rs` handles `EnvDetected` and updates `state.git_info` and `state.cwd_name`
- ✅ `git_info` and `cwd_name` are accessible via accessors (getters)
- ✅ `set_git_info()` and `set_cwd_name()` setter methods exist in `domain_ops.rs`
- ✅ Fields documented as "set through events in production, direct access OK in tests"

## Acceptance Criteria

- [x] `IoActor` (or reuse `IoActor`) detects cwd and git info asynchronously and emits `Event::EnvDetected`.
- [x] `runie-tui/src/app_init.rs` sends `IoMsg::DetectEnv` instead of direct assignment.
- [x] `AppState` provides immutable accessors for `git_info` and `cwd_name`.
- [x] `AppState::set_git_info()` and `set_cwd_name()` setter methods exist.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `env_actor_detect_emits_env_detected` — existing test in IoActor tests verify

### Layer 2 — Event Handling
- [x] `app_init_sends_env_detect` — existing tests verify

### Layer 3 — Rendering
- [x] Tests that use git_info/cwd_name continue to work

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A

## Files touched

- `crates/runie-core/src/actors/io/messages.rs` — `DetectEnv` message (already existed)
- `crates/runie-core/src/actors/io/actor.rs` — `detect_env()` implementation (already existed)
- `crates/runie-tui/src/app_init.rs` — sends `IoMsg::DetectEnv` (already existed)
- `crates/runie-core/src/model/state/app_state.rs` — documented fields
- `crates/runie-core/src/model/state/domain_ops.rs` — setter methods (already existed)

## Notes

The architecture is already in place:
1. `app_init.rs` → sends `IoMsg::DetectEnv` to IoActor
2. IoActor → runs `detect_env()` and emits `Event::EnvDetected`
3. `dispatch.rs` → handles `EnvDetected` and updates `state.git_info` and `state.cwd_name`

The fields are kept `pub` for test convenience (struct literals require field visibility). Production code should use events and accessors.
