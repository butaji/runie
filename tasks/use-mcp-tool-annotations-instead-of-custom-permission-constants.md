# Use MCP tool annotations instead of custom permission constants

## Status

`todo`

## Context

`crates/runie-core/src/tool/schema.rs` and `crates/runie-core/src/tool/mod.rs` define custom `READ_ONLY`/`REQUIRES_APPROVAL` constants on `ToolDef`. `to_mcp_tool` maps only `read_only_hint`. The MCP standard already provides richer `ToolAnnotations` (`read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint`).

## Goal

Drop the custom constants and use `rmcp::model::ToolAnnotations` directly. The permission gate should read these annotations to decide allow/ask/deny.

## Acceptance Criteria

- [ ] Remove `READ_ONLY`/`REQUIRES_APPROVAL` constants from `ToolDef`.
- [ ] Add `ToolAnnotations` to the tool schema and MCP conversion.
- [ ] Update permission gate to inspect annotations.
- [ ] All existing permission decisions are preserved.

## Design Impact

No change to TUI element design or composition. Only tool permission metadata changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests mapping tool annotations to permission decisions.
- **Layer 2 — Event Handling:** `PermissionRequest` carries annotation-derived context.
- **Layer 3 — Rendering:** Permission dialog text is unchanged.
- **Layer 4 — E2E:** MCP tool registration preserves annotations.
- **Live tmux testing session (required):** A read-only tool does not prompt; a destructive tool prompts.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
