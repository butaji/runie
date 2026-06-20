# Consolidate duplicate setup across binaries

**Status**: done  
**Milestone**: R4  
**Category**: Architecture / Actors  
**Priority**: P1  

**Depends on**: pure-snapshot-and-tool-runtime-trait  
**Blocks**: none  

## Description

`runie-print`, `runie-json`, `runie-server`, and headless tests each construct their own system prompts, permission gates, and runtime setups. This task creates shared helpers in `runie-core` and moves `spawn_headless_runtime` out of `runie-provider`, eliminating duplication and ensuring all modes behave consistently.

## Acceptance Criteria

- [x] `runie-core/src/turn_setup.rs` provides `build_system_prompt` and `build_permission_gate` helpers.
- [x] All non-interactive binaries use the shared helpers.
- [x] `spawn_headless_runtime` moves from `runie-provider` to `runie-core`.
- [x] Provider crate no longer depends on core runtime setup logic in the wrong direction.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `system_prompt_is_identical_across_modes` — TUI, print, JSON, and server produce the same prompt for the same inputs.
- [x] `permission_gate_is_identical_across_modes` — headless and interactive gates share construction logic.

### Layer 2 — Event Handling
- [x] N/A — this task is about shared factories, not event flow.

### Layer 3 — Rendering
- [x] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [x] `headless_binary_uses_shared_runtime_factory` — `runie-print` exercises the shared runtime builder.

## Files touched

- `crates/runie-core/src/turn_setup.rs` (new)
- `crates/runie-core/src/permissions/gate.rs` (new; PermissionGate moved from runie-agent)
- `crates/runie-core/src/headless_runtime.rs` (`spawn_headless_runtime` added)
- `crates/runie-provider/src/lib.rs`
- `crates/runie-print/src/main.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/lib.rs`
- `tasks/consolidate-binary-setup.md`

## Notes

- Keep the helpers pure: they take config/state values and return strings/structs, performing no IO.
- `runie-core` now owns `PermissionGate` and `spawn_headless_runtime`, breaking the provider→core→provider cycle.
- This task is lower priority than the layers above; it can be deferred if earlier phases require more iterations.
