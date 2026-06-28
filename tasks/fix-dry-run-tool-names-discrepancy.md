# Fix dry-run tool-name discrepancy

**Status**: done
**Milestone**: R5
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/dry_run.rs` returns a hard-coded list of tool names (`read`, `write`, `edit`, `bash`, `glob`, `grep`, `search`) that does not match the canonical `BUILTIN_TOOL_NAMES` in `crates/runie-core/src/tool/mod.rs`. The dry-run path should derive its list from the canonical source so it stays correct as tools are renamed or added.

## Acceptance Criteria

- [ ] `dry_run::core_tool_names()` uses `BUILTIN_TOOL_NAMES` (or a shared constant) instead of a hand-written list.
- [ ] The canonical source is the single place that lists built-in tool names.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dry_run_tool_names_match_canonical` — `core_tool_names()` equals `BUILTIN_TOOL_NAMES`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/dry_run.rs`
- `crates/runie-core/src/tool/mod.rs`

## Notes

- This is a one-line fix with high correctness value; do not expand scope.
