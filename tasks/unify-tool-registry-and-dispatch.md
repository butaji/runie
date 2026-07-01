# Unify tool registry and dispatch

## Status

`done`

## Context

Tool dispatch is duplicated in three places:
- `crates/runie-agent/src/tool_runner.rs:49-62`
- `crates/runie-agent/src/turn/mod.rs:235-257`
- `crates/runie-agent/src/headless/mod.rs:141-176`

`build_tool_registry` repeats the same list for OpenAI schema generation. Adding a tool requires edits in multiple files.

## Goal

Create a single `ToolRegistry: Vec<Box<dyn ToolDef>>` populated once and used for schema generation, dispatch, and headless execution.

## Acceptance Criteria

- [x] Define an object-safe `ToolDef` trait or a tool enum that supports name lookup and execution.
- [x] Populate the registry once at startup.
- [x] `tool_runner`, `turn`, and `headless` dispatch through the registry.
- [x] OpenAI/MCP schema generation iterates the registry.
- [x] All existing tool tests pass.

## Design Impact

No change to TUI element design or composition. Only internal tool dispatch behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for registry lookup and schema generation.
- **Layer 2 — Event Handling:** Tool execution events are unchanged.
- **Layer 3 — Rendering:** Tool result display is unchanged.
- **Layer 4 — E2E:** Provider replay fixture with multiple tools passes.
- **Live tmux validation:** A turn calling bash, read_file, and edit_file tools works end-to-end.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
