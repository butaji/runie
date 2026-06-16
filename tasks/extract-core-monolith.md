# Extract runie-core Monolith

**Status**: in-progress
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

## Progress

- Created `crates/runie-engine/` and moved all tool `call()` implementations from
  `crates/runie-core/src/tool/` into `crates/runie-engine/src/tool/`.
- Kept the `Tool` trait, `ToolContext`, `ToolOutput`, `ToolStatus`, `ToolRegistry`,
  `ToolCallState`/`ToolCallTracker`, and formatting helpers in `runie-core`.
- Moved `builtin_registry()` to `runie-engine::tool` and updated `runie-agent` to
  depend on `runie-engine` for built-in tool execution.
- Removed the built-in tool files from `runie-core`; `core_has_no_tool_impls` is
  now satisfied.
- `crates/runie-core/src/update/` and `crates/runie-core/src/dialog/` were **not**
  moved in this pass: they contain inherent `impl AppState` blocks and are heavily
  depended on by core modules/commands, so a clean move requires a larger refactor
  (trait-based dispatcher or moving callers out of core). Left in core and
  documented below.
- `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `core_compiles_without_ratatui` — `runie-core` has no `ratatui` dependency.
- [x] `core_has_no_tool_impls` — no tool `call()` implementations remain in core.

### Layer 2 — Event Handling
- [ ] `event_bus_still_functions` — actors publish/subscribe across crate boundary.

## Files touched

- `crates/runie-core/src/tool/impls` → `crates/runie-engine/src/tool/`
- `crates/runie-core/src/update/` → `crates/runie-engine/src/update/` (pending)
- `crates/runie-core/src/dialog/` → `crates/runie-engine/src/dialog/` (pending)
- `crates/runie-core/src/ui/` → `crates/runie-tui/src/ui/` (pending)
- `crates/runie-core/src/tests/` → split across crates (pending)
- `Cargo.toml`
- `crates/runie-core/build.rs`

## Notes

This is the largest R3 refactor. Do it incrementally to keep PRs reviewable; this task may spawn sub-tasks.

### Blockers for the remaining modules

- `update/` cannot move to `runie-engine` as-is because `update/mod.rs` contains
  `impl AppState { pub fn update(...) }`, which is an inherent impl on a type
  defined in `runie-core`. Rust's orphan rules prohibit inherent impls for a
  foreign type in another crate. A clean move requires either converting the
  dispatcher to a trait (callers would need to import it) or moving the callers
  out of `runie-core`, both of which are larger refactors.
- `dialog/` is used directly by many `runie-core` modules (commands, login flow,
  providers dialog, settings, tests). Moving it to `runie-engine` would create a
  dependency cycle unless those callers also move out of core. Leaving it in core
  for now.
- The lint allow-list in `crates/runie-core/build.rs` had its paths updated for
  the moved tool files (`search.rs`, `find_definitions.rs`). These are pre-existing
  oversized functions/files, not new violations.
