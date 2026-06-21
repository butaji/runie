# Tighten build-lint complexity heuristic

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/build_lint.rs::count_complexity` is the heuristic that enforces the 10-complexity ceiling. It counts `if`, `else if`, `match`, `while`, `for`, `&&`, `||`, `?`. It explicitly does not count `loop`, `break`, `continue`, or `return` because they appear in most async/production functions and would produce excessive false positives.

The cost of the lenient heuristic: nested `match` arms with hidden complexity slip through. Example: `update/agent/mod.rs::dispatch` is a flat `match` with 10 arms — each arm is a 1-line state mutation, but the function reads as a 10-step procedure and just barely hits the function-length limit. A stricter heuristic would catch it earlier.

## Acceptance Criteria

- [ ] One of:
  - Add a match-arm counter: each arm in a `match` contributes +1.
  - Cap individual match arms at a small length (e.g. 8 lines).
  - Add a brace-depth-aware variant of the existing heuristic.
- [ ] Existing compliant code still passes (`cargo build --workspace` exits 0).
- [ ] `update/agent/mod.rs::dispatch` is reported as over-complex or over-length under the new heuristic (validates the change has teeth).
- [ ] Document any false-positive-prone tokens that remain excluded.

## Tests

### Layer 1 — State/Logic
- [ ] Add unit tests in `build_lint.rs` covering the new tokens:
  - `match { A => ..., B => ..., C => ... }` counts as 1 match + 3 arms.
  - `nested if (a && b) { if c || d { ... } }` counts each branch point separately.
  - `loop { break; }` still excluded (or document the change).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo build --workspace` exits 0.
- [ ] `RUNIE_SKIP_BUILD_CHECKS=1 cargo test --workspace` runs the full suite.

## Files touched

- `crates/runie-core/build_lint.rs`
- `crates/runie-core/src/build_lint.rs` (the included shared implementation)

## Notes

- The heuristic is explicitly documented as approximate. A real cyclomatic-complexity tool (e.g. `tokei`, `complexity`) would be more accurate but adds a build dependency. Prefer extending the local heuristic to keep build dependency-free.
