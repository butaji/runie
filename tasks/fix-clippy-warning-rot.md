# Fix Clippy Warning Rot

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`cargo clippy --workspace` emits 233 warnings, 126 from `runie-core` alone. Categories include unused imports, dead functions, dead types/fields, and unused variables. Noise hides real bugs.

## Acceptance Criteria

- [ ] Workspace clippy warnings are reduced below the 233 baseline.
- [ ] Dead code is deleted or explicitly allowed with a comment.
- [ ] Unused imports are removed.
- [ ] Intentionally kept stubs are documented or gated.

## Tests

### Layer 1 — State/Logic
- [ ] `clippy_count_below_baseline` — `cargo clippy --workspace` warning count < 233.

### Layer 2 — Event Handling
- [ ] `cargo_test_passes` — no test broken by dead-code removal.

## Files touched

- Workspace-wide; concentrated in `crates/runie-core/src/`

## Notes

Run `cargo clippy --fix` as a first pass, then manually review remaining warnings.
