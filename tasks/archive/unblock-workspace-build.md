# Unblock Workspace Build

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

`crates/runie-core/build.rs` enforces a 500-line file limit, 40-line function limit, and complexity limit of 10. Several files created during earlier R3 work exceed these limits, so `cargo test --workspace` fails at the custom build script before reaching rustc. This task unblocks the build by splitting or simplifying the offending files.

## Acceptance Criteria

- [ ] `cargo test --workspace` compiles past the custom build-script lint.
- [ ] The build script still enforces 500/40/10 limits.
- [ ] No oversized files remain in `crates/`.
- [ ] `cargo test --workspace` then passes all tests.

## Tests

### Layer 1 — State/Logic
- [ ] `lint_file_length_passes` — build script reports zero file-length violations.
- [ ] `lint_function_length_passes` — build script reports zero function-length violations.

### Layer 2 — Event Handling
N/A — build lint only.

### Layer 3 — Rendering
N/A — build lint only.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` passes in CI.

## Files touched

- `crates/runie-core/build.rs`
- `crates/runie-core/src/tool/mod.rs`
- Any other files flagged by the lint script.

## Notes

- This is a short-term unblock task. Long-term lint discipline remains 500/40/10.
- Do **not** raise the build-script thresholds as a workaround; split or simplify the code instead.
