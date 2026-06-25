# Consolidate `tests/login_logout/` thin test files

**Status**: done
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P3

**Depends on**: consolidate-login-flow-handlers
**Blocks**: none

## Description

`tests/login_logout/` has 14 files totaling 1,798 LOC — 12% of all core test code. The four smallest (`multiple` 32, `edge_cases` 34, `model_select` 47, `core` 49 = 162 LOC) are thin slivers that fragment test coverage. Fold them into their natural parents: `model_select` + `model_select_edge_cases` → one `model_select.rs`; `core` + `state_machine` → one `core.rs`; `multiple` + `edge_cases` → folded into `core.rs`. Result: 14 files → 10.

## Acceptance Criteria

- [x] `tests/login_logout/model_select.rs` (47) merged with `model_select_edge_cases.rs` (200) → single `model_select.rs` (237 LOC).
- [x] `tests/login_logout/core.rs` (49) merged with `state_machine.rs` (110) + `multiple.rs` (32) + `edge_cases.rs` (34) → single `core.rs` (208 LOC).
- [x] `model_select_edge_cases.rs`, `state_machine.rs`, `multiple.rs`, `edge_cases.rs` deleted.
- [x] `tests/login_logout/mod.rs` updated to declare the reduced module list.
- [x] All 24 test functions preserved (12 in `model_select.rs`, 12 in `core.rs`).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — test-only reorganization.

### Layer 2 — Event Handling
- [ ] `all_login_logout_tests_still_pass` — full `tests/login_logout/` suite green after merge (same test count, same assertions).

### Layer 3 — Rendering
- [ ] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [ ] N/A — test code only.

## Files touched

- `crates/runie-core/src/tests/login_logout/model_select.rs` — absorb `model_select_edge_cases.rs`
- `crates/runie-core/src/tests/login_logout/model_select_edge_cases.rs` → delete
- `crates/runie-core/src/tests/login_logout/core.rs` — absorb `state_machine.rs`
- `crates/runie-core/src/tests/login_logout/state_machine.rs` → delete
- `crates/runie-core/src/tests/login_logout/happy_path.rs` (or `core.rs`) — absorb `multiple.rs` + `edge_cases.rs`
- `crates/runie-core/src/tests/login_logout/multiple.rs` → delete
- `crates/runie-core/src/tests/login_logout/edge_cases.rs` → delete
- `crates/runie-core/src/tests/login_logout/mod.rs` — remove deleted module declarations

## Notes

Depends on `consolidate-login-flow-handlers` so the handler module layout is stable before reorganizing its tests. Verify the merge targets stay under 500 lines (test files are exempt from the build.rs linter, but keep them navigable). Do NOT merge `login_flow.rs` (352) or `happy_path.rs` (247) — they're already substantial. The goal is eliminating 4 sub-50-LOC sliver files, not aggressive consolidation. Run the test suite before and after, compare test counts: `cargo test --workspace 2>&1 | grep "test result"` should show identical pass counts.
