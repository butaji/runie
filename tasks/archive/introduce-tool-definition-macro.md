# Introduce tool-definition macro

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Every tool in `crates/runie-engine/src/tool/*.rs` hand-writes an `input_schema(&self) -> Value` implementation returning `json!({ "type": "object", "properties": { ... }, "required": [...] })`. Adding a common field or `additionalProperties: false` requires touching every tool file. Names and read-only/approval flags are also repeated.

## Acceptance Criteria

- [ ] A declarative macro or builder generates `name()`, `description()`, `input_schema()`, `is_read_only()`, and `requires_approval()` from one declaration.
- [ ] All tools in `crates/runie-engine/src/tool/*.rs` use the macro/builder.
- [ ] Generated schemas are byte-for-byte equivalent to the hand-written ones (or intentionally improved).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `macro_generates_bash_tool_schema` — generated schema matches the current hand-written schema.
- [ ] `macro_generates_read_only_tool_flags` — `is_read_only` and `requires_approval` derive from declaration.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_tool_schemas_unchanged` — a headless turn with tool calls still produces the same provider request shape.

## Files touched

- `crates/runie-engine/src/tool/bash.rs`
- `crates/runie-engine/src/tool/grep.rs`
- `crates/runie-engine/src/tool/edit_file.rs`
- `crates/runie-engine/src/tool/list_dir.rs`
- `crates/runie-engine/src/tool/read_file.rs`
- `crates/runie-engine/src/tool/write_file.rs`
- `crates/runie-engine/src/tool/fetch_docs.rs`
- `crates/runie-engine/src/tool/search/core.rs`
- `crates/runie-engine/src/tool/find_definitions.rs`
- New `crates/runie-engine/src/tool/define.rs` or similar.

## Notes

Prefer a declarative macro if possible so tool schemas remain visible at compile time and rust-analyzer can expand them.
