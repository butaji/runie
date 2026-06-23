# Document or Improve build.rs Complexity Heuristic

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`count_complexity` in `build.rs` only counts `if`, `else if`, `match`, `while`, `for`, `&&`, `||`, and `?`. It misses `loop`, `break`, `continue`, `return` in guards, nested closures, `try` blocks, and other constructs.

## Acceptance Criteria

- [ ] Document the heuristic as approximate in `AGENTS.md` and `docs/SPEC.md`.
- [ ] OR replace it with a proper AST-based lint (e.g. clippy complexity lints) if feasible.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `complexity_count_includes_loops` — `loop`/`break`/`continue` are counted.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/build.rs`
- `AGENTS.md`
- `docs/SPEC.md`

## Notes

The current heuristic is intentionally lightweight. Documenting its limits is the minimum acceptable fix.
