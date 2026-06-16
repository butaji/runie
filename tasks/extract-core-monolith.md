# Extract runie-core Monolith

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: unify-tool-implementations
**Blocks**: (none)

## Description

`runie-core` has grown to 53,396 lines and owns responsibilities that belong elsewhere: tool execution, update dispatch, dialog DSL, UI element AST, actors, orchestrator, and ~19k lines of integration tests. `docs/SPEC.md` says core should be "Events, AppState, sessions, config, commands".

## Acceptance Criteria

- [ ] `runie-core` is reduced to types, events, state primitives, sessions, config, and commands.
- [ ] Tool execution moves to `runie-agent`.
- [ ] Update dispatch and dialog/form logic move to a new `runie-engine` crate or into `runie-agent`.
- [ ] UI rendering AST moves to `runie-tui`.
- [ ] Integration tests move to the crate that owns the behavior.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `core_compiles_without_ratatui` — `runie-core` has no `ratatui` dependency.
- [ ] `core_has_no_tool_impls` — no tool `call()` implementations remain in core.

### Layer 2 — Event Handling
- [ ] `event_bus_still_functions` — actors publish/subscribe across crate boundary.

## Files touched

- `crates/runie-core/src/tool/` → `crates/runie-agent/src/tool/`
- `crates/runie-core/src/update/` → `crates/runie-engine/src/update/` (or `runie-agent`)
- `crates/runie-core/src/dialog/` → `crates/runie-engine/src/dialog/`
- `crates/runie-core/src/ui/` → `crates/runie-tui/src/ui/`
- `crates/runie-core/src/tests/` → split across crates
- `Cargo.toml`
- `docs/SPEC.md`

## Notes

This is the largest R3 refactor. Do it incrementally to keep PRs reviewable; this task may spawn sub-tasks.
