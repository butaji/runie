# FFF Query Syntax and Examples

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: fff-unified-search-tool
**Blocks**: (none)

## Description

Teach agents (and users) to use FFF’s constraint query language (`*.rs !test/`, `git:modified`, `foo bar`). Add examples to the unified `search` tool schema and documentation.

## Acceptance Criteria

- [ ] Tool schema `examples` include constraint queries.
- [ ] Agent prompt guidance recommends constraint syntax for broad searches.
- [ ] User docs cover query syntax.
- [ ] Query parser handles quoted strings and negation.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `query_parser_applies_glob_constraint` — `*.rs` filters correctly.
- [ ] `query_parser_applies_negation` — `!test/` excludes correctly.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/search.rs`
- `docs/usage.md` or `docs/SPEC.md`

## Notes

- This is mostly schema/docs work; parsing is handled by `fff-search`.
- See `docs/adr/0023-fff-search-integration.md`.
