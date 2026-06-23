# Remove Legacy Tool Enum and Consolidate Bash Logic

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

There are at least three tool abstractions in flight: the old `runie_agent::tools::Tool` enum (with `execute()`), the new `runie_core::tool::Tool` trait used by `runie_engine`, and an unused `ToolRuntime` trait. The old `runie-agent/src/tools/` modules are compiled but the main execution path uses `runie_engine::tool::builtin_registry()`. Bash execution logic is duplicated across `runie-agent/src/tools/bash.rs`, `runie-engine/src/tool/bash.rs`, and `runie-core/src/update/tools.rs`.

## Acceptance Criteria

- [ ] Delete the legacy `runie_agent::tools::Tool` enum and its `exec.rs`, `fs.rs`, `bash.rs`, etc.
- [ ] Remove unused `ToolRuntime` trait if it is not intended for use.
- [ ] Consolidate bash execution into a single implementation in `runie-engine`.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `builtin_registry_contains_expected_tools` — expected tools still present after cleanup.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_bash_tool_still_works` — bash tool executes after consolidation.

## Files touched

- `crates/runie-agent/src/tools/`
- `crates/runie-engine/src/tool/bash.rs`
- `crates/runie-core/src/tool/runtime.rs`
- `crates/runie-core/src/update/tools.rs`

## Notes

Verify no hidden callers of the legacy enum before deleting. This is cleanup debt from the R3 unification pass.
