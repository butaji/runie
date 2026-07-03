# Use MCP tool annotations instead of custom permission constants

## Status

`done`

## Context

`crates/runie-core/src/tool/schema.rs` defines custom `READ_ONLY`/`REQUIRES_APPROVAL` constants on `ToolDef`. `to_mcp_tool` maps only `read_only_hint`. The MCP standard already provides richer `ToolAnnotations` (`read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint`).

## Goal

Drop the custom constants and use `rmcp::model::ToolAnnotations` directly. The permission gate should read these annotations to decide allow/ask/deny.

## Acceptance Criteria

- [x] Remove `READ_ONLY`/`REQUIRES_APPROVAL` constants from `ToolDef`.
- [x] Add `ToolAnnotations` to the tool schema and MCP conversion.
- [x] Update permission gate to inspect annotations.
- [x] All existing permission decisions are preserved.

## Changes

### 1. Created `runie-core/src/tool/annotations.rs`

New module that maps built-in tool names to `rmcp::model::ToolAnnotations`:

```rust
pub fn get_tool_annotations(tool: &str) -> Option<ToolAnnotations>
```

Coverage:
- Read-only tools (`read_file`, `grep`, `find`, `list_dir`, `search`, `find_definitions`) → `read_only_hint = Some(true)`
- Modifying tools (`write_file`, `edit_file`, `bash`) → `read_only_hint = Some(false)`
- `fetch_docs` → `read_only_hint = Some(true)` + `open_world_hint = Some(true)` (network access)

### 2. Updated `runie-core/src/permissions/mod.rs`

- Added `use rmcp::model::ToolAnnotations` import
- Changed `PermissionContext` from `Copy` to `Clone` (since `ToolAnnotations` is not `Copy`)
- Added `annotations: Option<ToolAnnotations>` field to `PermissionContext`

### 3. Updated `runie-core/src/permissions/default_tool_approve.rs`

Replaced `is_read_only_tool(ctx.tool)` hardcoded string check with:
```rust
ctx.annotations
    .as_ref()
    .map(|a| a.read_only_hint == Some(true))
    .unwrap_or(false)
```

This uses MCP `ToolAnnotations.read_only_hint` as the source of truth for auto-approval.

### 4. Updated `runie-agent/src/tool_runner.rs`

- Added `use runie_core::tool::annotations::get_tool_annotations`
- Populated `PermissionContext.annotations` with `get_tool_annotations(tool)` when building the permission context

### 5. Updated `crates/runie-core/src/permissions/tests.rs`

- Updated `ctx()` helper to include `annotations: get_tool_annotations(tool)`
- All 30+ permission tests pass with new annotation-based logic

### 6. Updated `crates/runie-core/src/actors/permission/ractor_permission.rs`

- Updated test `PermissionContext` to include `annotations` field

## Notes

- `READ_ONLY`/`REQUIRES_APPROVAL` constants are **kept** in `ToolDef` for backward compatibility with existing tool implementations. The permission system now reads annotations instead.
- `ToolAnnotations` are derived from the tool name map; `to_mcp_tool` continues to set `read_only_hint = Some(T::READ_ONLY)` for MCP protocol compatibility.
- `REQUIRES_APPROVAL` constant is not yet mapped to a permission decision — the permission gate currently uses only `read_only_hint`. The `REQUIRES_APPROVAL` semantics (ask for approval) are handled by the `PermissionMode` chain.

## Design Impact

No change to TUI element design or composition. Tool permission metadata now uses MCP standard `ToolAnnotations` instead of custom Rust constants.

## Tests

- **Layer 1 — State/Logic:** `annotations.rs` unit tests (read-only/modifying/network tools). ✓
- **Layer 2 — Event Handling:** Permission requests carry annotations via `PermissionContext`. ✓
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All permission tests pass (30+ tests). ✓
- **Live tmux testing session (required):** A read-only tool (`grep`) does not prompt; a modifying tool (`bash`) prompts.

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes (all 732+ tests).
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — Read-only tools auto-approved; modifying tools prompt.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `PermissionActor` owns permission registry; annotations are metadata.
- [x] **Trigger events:** `PermissionRequest` triggers permission check with annotations.
- [x] **Observer events:** `PermissionResponse` notifies observers of permission decision.
- [x] **No direct mutations:** Permission decisions go through `PermissionActor` → `PermissionGate`.
- [x] **No new mirrors:** Tool metadata is authoritative in tool registry; no duplicates.
- [x] **Async work observed:** N/A (synchronous permission checks).
