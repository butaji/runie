# Delete or merge the inspector tool pipeline

**Status**: todo
**Milestone**: R2
**Category**: Agent / Tools
**Priority**: P1

**Depends on**: centralize-built-in-tool-names
**Blocks**: none

## Description

`crates/runie-agent/src/inspector.rs` defines its own `dispatch_tool`, `run_tool`, `unknown_tool_output`, and `ToolPipeline` inspector trait. This mirrors `crates/runie-agent/src/tool_runner.rs::execute_tool_call` and the existing skill before/after hooks. `inspector.rs` is public in `lib.rs` but has no in-tree callers. It should either be deleted or folded into `tool_runner.rs` so skill hooks and inspectors share one dispatch path and one canonical `BUILTIN_TOOL_NAMES` lookup.

## Acceptance Criteria

- [ ] Verify whether any external crate/binary depends on `runie_agent::inspector::ToolPipeline`.
- [ ] If no external consumers exist, delete `inspector.rs` and remove it from `lib.rs`.
- [ ] If external consumers exist, merge `inspector.rs` dispatch into `tool_runner.rs` so there is one `BUILTIN_TOOL_NAMES` lookup and one hook invocation path.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `single_tool_dispatch_path` — after the change, only one set of dispatch/run helpers remains.
- [ ] `builtin_tool_names_lookup_used_everywhere` — no literal tool-name lists remain in the merged path.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `inspector_or_merged_hooks_still_fire` — skill before/after hooks still dispatch correctly.

## Files touched

- `crates/runie-agent/src/inspector.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/lib.rs`

## Notes

- Coordinate with `centralize-built-in-tool-names.md` so the merged path uses the canonical `runie_core::tool::BUILTIN_TOOL_NAMES`.
- If `inspector.rs` is kept for debugging/tracing, make it a thin wrapper around `tool_runner.rs` rather than a parallel implementation.
