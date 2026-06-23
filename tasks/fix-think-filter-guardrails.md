# Verify `think_filter.rs` build guardrail compliance

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-agent/src/think_filter.rs` was refactored since the original review. The production code is now under the 500-line file limit and every production function is under 40 lines with complexity ≤ 10. This task is to verify that the guardrail compliance holds and that test coverage is sufficient.

## Acceptance Criteria

- [ ] `cargo build --workspace` passes with zero lint violations.
- [ ] `cargo test -p runie-agent think_filter` passes.
- [ ] Existing `ThinkFilter` behavior is preserved.

## Tests

### Layer 1 — State/Logic
- [ ] Existing tests in `crates/runie-agent/src/tests/think_filter.rs` pass.
- [ ] If the inline `inner_tests` module has not been moved, ensure it remains small and does not push the file over the limit.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-agent/src/think_filter.rs` — verify only.
- `crates/runie-agent/src/tests/think_filter.rs` — verify tests pass.

## Implementation

### Step 1: Verify limits

Run:

```bash
cargo build --workspace
cargo test -p runie-agent think_filter
```

Expected: build passes, all tests pass.

### Step 2: If the build fails, fix the reported violation

If a future change reintroduces a guardrail violation, split the offending function or move the test module to `crates/runie-agent/src/tests/think_filter.rs`.

### Step 3: Commit (if any changes)

Only needed if fixes are required:

```bash
git add crates/runie-agent/src/think_filter.rs crates/runie-agent/src/tests/think_filter.rs tasks/fix-think-filter-guardrails.md
git commit -m "fix(agent): keep think_filter within build guardrails"
```

## Notes

- The original 197-line `feed_text` function has already been decomposed into `feed_text`, `consume_outside`, `consume_inside`, and small handler helpers.
- Do not reintroduce large nested functions.
