# Fix `cargo fmt` Violations

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

## Description

`cargo fmt --all -- --check` currently reports formatting diffs across the workspace. This will break the CI `fmt` job.

## Acceptance Criteria

- [ ] `cargo fmt --all` applied.
- [ ] `cargo fmt --all -- --check` passes with no diffs.
- [ ] No functional changes introduced.
- [ ] `cargo test --workspace` still passes.

## Tests

### Layer 4 — Smoke
- [ ] CI fmt step would pass.
