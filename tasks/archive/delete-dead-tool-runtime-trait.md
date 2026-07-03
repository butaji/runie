# Delete dead `runie_macros::define_tool!` proc macro

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: collapse-tool-runtime-traits

## Description

The `define_tool!` proc macro in `runie-macros/src/tool.rs` is dead code. It is never used anywhere in the codebase — all tool definitions in `runie-engine/src/tool/*.rs` use `crate::define_tool!` (the `#[macro_export]` macro in `runie-engine/src/tool/define.rs`) instead. The proc macro's generated code references `::runie_core::tool::runtime::ToolRuntime`, `ExecApprovalRequirement`, and `ToolError` from a path that no longer exists.

Delete:
- `crates/runie-macros/src/tool.rs`
- The `tool` module from `crates/runie-macros/src/lib.rs`

After deletion, verify `runie-engine` still compiles (it uses `crate::define_tool!` not `runie_macros::define_tool!`).

## Acceptance criteria

- [x] `crates/runie-macros/src/tool.rs` deleted.
- [x] `tool` module removed from `crates/runie-macros/src/lib.rs`.
- [x] `runie-engine` still compiles with `cargo check -p runie-engine`.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- N/A (deletion only).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `runie_engine_still_builds` — `cargo check -p runie-engine` succeeds after deletion.

## Files touched

- `crates/runie-macros/src/tool.rs` (deleted)
- `crates/runie-macros/src/lib.rs` (remove `mod tool`)

## Notes

The `runie-engine` crate uses its own `define_tool!` macro (`#[macro_export]` in `src/tool/define.rs`) which implements the `Tool` trait methods (`name`, `description`, `input_schema`, `is_read_only`, `requires_approval`) without referencing any `ToolRuntime` path. The proc macro in `runie-macros` is a separate, unused implementation.
