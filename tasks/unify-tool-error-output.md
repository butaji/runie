# Unify Tool Error Output Functions

**Status**: done
**Completed**: 2026-06-16
**Notes**: Added `runie_core::tool::tool_error` helper and updated `list_dir`, `edit_file`, and `grep` tool implementations in `runie-engine`. Added 2 Layer 1 tests. cargo test --workspace passes.
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: `unify-resolve-path`
**Blocks**: (none)

## Description

Consolidate 3 variants of error output functions into a single shared utility.

**Variants:**
- `list_dir.rs:86` — `error_output(path, e, start)`
- `edit_file.rs:125` — `error_output(msg, start)`
- `grep.rs:129` — `error_result(msg, start)`

**Proposed unified API:**
```rust
pub fn tool_error(tool_name: &str, msg: &str, is_warning: bool) -> ToolOutput {
    // ... consolidated implementation
}
```

## Acceptance Criteria

- [ ] Unified `tool_error` function in `crates/runie-core/src/tool/mod.rs`.
- [ ] All 3 files updated to use shared function.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_error_returns_error_output` — returns error ToolOutput.
- [ ] `tool_error_warning_flag` — warning variant works.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/list_dir.rs`
- `crates/runie-core/src/tool/edit_file.rs`
- `crates/runie-core/src/tool/grep.rs`
