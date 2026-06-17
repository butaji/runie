# FFF Query Syntax and Examples

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-unified-search-tool
**Blocks**: (none)

## Description

Teach agents (and users) to use FFF’s constraint query language (`*.rs !test/`, `git:modified`, `foo bar`). Add examples to the unified `search` tool schema and documentation.

## Acceptance Criteria

- [x] Tool schema `examples` include constraint queries.
- [x] Agent prompt guidance recommends constraint syntax for broad searches. (in SPEC.md)
- [x] User docs cover query syntax. (in docs/SPEC.md)
- [x] Query parser handles quoted strings and negation. (FFFQuery constraints)
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `query_parser_applies_glob_constraint` — `*.rs` parses as Extension constraint.
- [x] `query_parser_applies_negation` — `!test/` parses as Not constraint.
- [x] `query_parser_handles_git_status_filter` — `git:modified` parses as GitStatus constraint.
- [x] `query_parser_handles_location_hint` — `lib.rs:42` parses as Location::Line.
- [x] `query_parser_handles_location_with_column` — `lib.rs:42:5` parses as Location::Position.
- [x] `query_parser_handles_mixed_query` — combined query parses all constraint types.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [x] `search_tool_schema_has_examples` — schema examples cover glob, negation, git, location.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/search.rs`
- `docs/usage.md` or `docs/SPEC.md`

## Notes

- This is mostly schema/docs work; parsing is handled by `fff-search`.
- See `docs/adr/0023-fff-search-integration.md`.
