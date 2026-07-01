# Fix dry-run tool-name discrepancy

**Status**: done
**Note**: Verified 2026-06-29 — `dry_run.rs::core_tool_names()` delegates to `BUILTIN_TOOL_NAMES` and test passes.
**Milestone**: R5
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/dry_run.rs` returns a hard-coded list of tool names (`read`, `write`, `edit`, `bash`, `glob`, `grep`, `search`) that does not match the canonical `BUILTIN_TOOL_NAMES` in `crates/runie-core/src/tool/mod.rs`. The dry-run path should derive its list from the canonical source so it stays correct as tools are renamed or added.

## Acceptance Criteria

- [x] `dry_run::core_tool_names()` uses `BUILTIN_TOOL_NAMES` (or a shared constant) instead of a hand-written list.
- [x] The canonical source is the single place that lists built-in tool names.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dry_run_tool_names_match_canonical` — `core_tool_names()` equals `BUILTIN_TOOL_NAMES`.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/dry_run.rs`
- `crates/runie-core/src/tool/mod.rs`

## Notes

- This is a one-line fix with high correctness value; do not expand scope.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
