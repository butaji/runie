# FFF Find Definitions Tool

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-unified-search-tool
**Blocks**: (none)

## Description

Add an agentic `find_definitions` tool that uses FFF’s definition classifier to locate symbol definitions (`struct`, `fn`, `class`, `def`, `impl`, etc.). This is more precise than grepping for a name.

## Acceptance Criteria

- [x] New `find_definitions` tool registered in the tool registry.
- [x] Tool accepts `symbol` and optional `glob` filters.
- [x] Results include path, line number, column, and definition kind.
- [x] Uses FFF content search with `classify_definitions: true` and `is_definition` filtering.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `detect_kind_struct` — detects `struct` definitions.
- [x] `detect_kind_fn` — detects `fn` definitions.
- [x] `detect_kind_def` — detects `def` definitions.
- [x] `find_definitions_tool_schema` — schema has symbol/glob/path/limit fields.
- [x] `find_definitions_tool_name` — tool name is "find_definitions".
- [x] `find_definitions_tool_is_read_only` — tool is read-only.
- [x] `find_definitions_tool_no_approval` — no approval required.
- [x] `find_definitions_tool_description_mentions_classifier` — description references definition classifier.
- [x] `find_definitions_uninitialized_returns_error` — graceful error when FFF not running.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/find_definitions.rs` (new)
- `crates/runie-core/src/tool/mod.rs`

## Notes

- Consider returning a small snippet around each definition.
- See `docs/adr/0023-fff-search-integration.md`.
