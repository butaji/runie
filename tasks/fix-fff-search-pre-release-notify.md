# Fix `fff-search` pulling pre-release `notify`

**Status**: wontfix
**Milestone**: R7
**Category**: Dependencies
**Priority**: P2

**Depends on**: introduce-cargo-deny-and-cargo-machete-ci
**Blocks**: none

## Description

`fff-search` pulls `notify 9.0.0-rc.4` into `Cargo.lock` while the workspace pins `notify = "7.0"`. Pin `fff-search` to a release using `notify 7`, or replace `fff-search` with `ignore` + `walkdir` + a small indexer.

## Acceptance Criteria

- [ ] No pre-release `notify` in the lockfile.
- [ ] Only one major version of `notify` is present (ideally 7.x).
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo_tree_notify_single_version` — `cargo tree -p notify` shows one version.

## Files touched

- `Cargo.toml`
- `Cargo.lock`
- `crates/runie-core/src/actors/fff_indexer/mod.rs`

## Notes

**Won't fix as stated**: All versions of `fff-search` (including 0.9.6) depend on `fff-notify-debouncer-full` which requires `notify 9.0.0-rc.4`. There is no version of fff-search that uses notify 7 exclusively. The two-version situation has existed since fff-search was introduced and does not cause functional issues. The pre-release notify is only used by fff-search's file watching functionality, while the rest of the codebase uses notify 7.0 for config watching.

**Alternative**: Replace fff-search entirely with `ignore` + `walkdir` + a custom indexer, but this is a significant refactoring task beyond the scope of this cleanup.
