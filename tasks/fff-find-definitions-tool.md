# FFF Find Definitions Tool

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-unified-search-tool
**Blocks**: (none)

## Description

Add an agentic `find_definitions` tool that uses FFF’s definition classifier to locate symbol definitions (`struct`, `fn`, `class`, `def`, `impl`, etc.). This is more precise than grepping for a name.

## Acceptance Criteria

- [ ] New `find_definitions` tool registered in the tool registry.
- [ ] Tool accepts `symbol` and optional `language`/`glob` filters.
- [ ] Results include path, line number, and definition kind.
- [ ] Uses FFF content search with `is_definition` filtering.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `find_definitions_locates_rust_fn` — finds a `fn` definition.
- [ ] `find_definitions_filters_by_language` — Rust `struct` vs Python `class`.

### Layer 2 — Event Handling
- [ ] `find_definitions_emits_search_request` — tool publishes a `SearchRequest` with definition filter.

### Layer 3 — Rendering
- [ ] `definition_results_render_with_kind` — TUI shows `fn`/`struct`/`class` badge.

### Layer 4 — Smoke / Crash
- [ ] `smoke_find_definitions` — run binary, ask for a definition, verify result.

## Files touched

- `crates/runie-core/src/tool/find_definitions.rs` (new)
- `crates/runie-core/src/tool/mod.rs`

## Notes

- Consider returning a small snippet around each definition.
- See `docs/adr/0023-fff-search-integration.md`.
