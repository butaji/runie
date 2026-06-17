# Align Build.rs Lint Thresholds (Strict Enforcement)

**Status**: in_progress
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P0

**Depends on**: (all simplification tasks)
**Blocks**: (none)

## Description

**ENFORCEMENT IS NOW ACTIVE.** `crates/runie-core/build.rs` enforces strict lint thresholds with **NO exceptions**.

| Threshold | Value |
|-----------|-------|
| File lines | 500 |
| Function lines | 40 |
| Complexity | 10 |

## Current Violations

File-length enforcement is now strict (no files over 500 lines).

### Functions over limits (allowed temporarily):
See `cargo build` output for the current `ALLOWED_FUNC_VIOLATIONS` list. These remain while the R4 simplification tasks (e.g. `adopt-tool-runtime-trait`, `adopt-permission-policy-chain`) refactor the remaining long functions.

## Acceptance Criteria

- [x] `ALLOWED_FILE_VIOLATIONS` removed from `build.rs` (now empty) ✓
- [ ] `ALLOWED_FUNC_VIOLATIONS` removed from `build.rs` (pending remaining R4 refactors)
- [ ] `is_allowed_func()` and related logic removed (pending)
- [x] File-length violations cause `cargo build` to fail ✓
- [x] `cargo build --workspace` succeeds (function violations are currently allowed)

## Tasks That Fix Violations

| Task | Fixes |
|------|-------|
| `unify-resolve-path` | Reduces tool file sizes |
| `unify-tool-error-output` | Consolidates error handling |
| `unify-agent-status-enum` | Simplifies orchestrator.rs, state.rs |
| `split-large-files` | Splits all files over 500 lines |
| `extract-search-item-builder` | Simplifies search.rs |
| All simplification tasks | Reduce complexity, fix violations |

## Files touched

- `crates/runie-core/build.rs` ✓ (enforcement active)

## Notes

Enforcement is active. Fix violations via simplification tasks.
Build will fail until all 80 violations are resolved.
