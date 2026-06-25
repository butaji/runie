# Tighten build-lint complexity heuristic

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/build_lint.rs::count_complexity` is the heuristic that enforces the 10-complexity ceiling. It counts `if`, `else if`, `match`, `while`, `for`, `&&`, `||`, `?`. It explicitly does not count `loop`, `break`, `continue`, or `return` because they appear in most async/production functions and would produce excessive false positives.

The cost of the lenient heuristic: nested `match` arms with hidden complexity slip through. Example: `update/agent/mod.rs::dispatch` is a flat `match` with 10 arms — each arm is a 1-line state mutation, but the function reads as a 10-step procedure and just barely hits the function-length limit. A stricter heuristic would catch it earlier.

## Acceptance Criteria

- [x] One of:
  - [x] Add a match-arm counter: each arm in a `match` contributes +1.
  - [ ] Cap individual match arms at a small length (e.g. 8 lines).
  - [ ] Add a brace-depth-aware variant of the existing heuristic.
- [x] Existing compliant code still passes (`cargo build --workspace` exits 0).
- [x] `update/agent/mod.rs::dispatch` is reported as over-complex or over-length under the new heuristic (validates the change has teeth).
- [x] Document any false-positive-prone tokens that remain excluded.

## Tests

### Layer 1 — State/Logic
- [x] Add unit tests in `build_lint.rs` covering the new tokens:
  - `match { A => ..., B => ..., C => ... }` counts as 1 match + 3 arms.
  - `nested if (a && b) { if c || d { ... } }` counts each branch point separately.
  - `loop { break; }` still excluded (or document the change).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo build --workspace` exits 0.
- [x] `RUNIE_SKIP_BUILD_CHECKS=1 cargo test --workspace` runs the full suite.

## Files touched

- `crates/runie-core/build_lint.rs`
- `crates/runie-core/src/build_lint.rs` (the included shared implementation)

## Notes

- Chose the brace-depth-aware variant: match arms (`=>`) are counted only when `brace_depth == 1` (top-level function body). At depth 2+ (inside closures, match blocks, etc.), arms are not counted, avoiding false positives from map literals and closure patterns.
- `count_complexity` now takes a `brace_depth` parameter. The `FnTracker` in `build.rs` passes the current depth to each line's complexity count.
- Complexity is counted BEFORE applying brace changes, so `match {` arms are at depth 1 (before the `{` opens depth 2) and the closing `}` of a block is counted at that block's depth.
- Remaining excluded tokens (documented in `build_lint.rs`): `loop`, `break`, `continue`, `return`. Closure patterns (`|| x =>`) are counted at depth 1 as a known limitation — the heuristic cannot distinguish them from match arms without a parser.
- `count_complexity` itself is kept under the complexity limit by using a flat return statement with no nested `if` chains (the `?` operator is excluded to keep the function lean).
