# Fix `fff-search` pulling pre-release `notify`

**Status**: todo
**Milestone**: R7
**Category**: Dependencies
**Priority**: P2

**Depends on**: introduce-cargo-deny-and-cargo-machete-ci
**Blocks**: none

## Description

`fff-search 0.9.6` pulls `notify 9.0.0-rc.4` into `Cargo.lock` while the workspace pins `notify = "7.0"`. Pin `fff-search` to a release using `notify 7`, or replace `fff-search` with `ignore` + `walkdir` + a small indexer.

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

- Coordinate with `fix-500-line-file-limit-violations.md` if fff_indexer is split.
