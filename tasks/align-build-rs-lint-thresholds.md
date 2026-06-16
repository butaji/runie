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

## Current Violations (80 total)

### Files over 500 lines (8):
- `state.rs` (790), `planner.rs` (800), `theme.rs` (657), `harness_skills.rs` (684)
- `tool/search.rs` (683), `update/mod.rs` (631), `update/agent.rs` (592), `keybindings.rs` (571)

### Functions over limits (72):
See `cargo build` output for full list.

## Acceptance Criteria

- [x] `ALLOWED_FILE_VIOLATIONS` removed from `build.rs` ✓
- [x] `ALLOWED_FUNC_VIOLATIONS` removed from `build.rs` ✓
- [x] `is_allowed_func()` and related logic removed ✓
- [x] Any violation causes `cargo build` to fail ✓
- [ ] `cargo build --workspace` succeeds with no violations (pending)

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
