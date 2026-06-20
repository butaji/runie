# Delete crates/_archive/ directory

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/_archive/harness/` (874 LOC across `lib.rs`, `results.rs`, `runner.rs`, `graders/static_analysis.rs`) is dead. No `Cargo.toml` lists it as a workspace member or path dep; no live `mod harness`/`use ...harness` references exist (the live `harness_skills` module is unrelated). The `clean-archive` task was marked done but this subdir re-accumulated afterwards.

## Acceptance Criteria

- [ ] `crates/_archive/` deleted entirely.
- [ ] No `Cargo.toml` references `crates/_archive` as a path dep.
- [ ] `rg "crates/_archive|mod _archive|mod archive" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure deletion.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_workspace_builds_without_archive` — `cargo check --workspace` green after deletion.

## Files touched

- `crates/_archive/` (entire directory)

## Notes

Confirm with `git log -- crates/_archive/` that nothing live depends on it before deletion. Distinct from the live `harness_skills/` module in `runie-core`.
