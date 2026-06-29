# Fix 500-line file-limit violations

**Status**: todo
**Milestone**: R7
**Category**: Build / CI
**Priority**: P1

**Depends on**: replace-build-linter-with-clippy-ci
**Blocks**: none

## Description

AGENTS.md enforces a 500-line limit per `.rs` file. Several production files exceed it: `tool_markers/strip.rs` (503), `diff.rs` (519), `config/mod.rs` (550), `ractor_session_actor.rs` (551), `session/tree.rs` (560), `ractor_config.rs` (972). Split or refactor them to comply.

## Acceptance Criteria

- [ ] All production `.rs` files are ≤ 500 lines.
- [ ] `scripts/check-file-limits.sh` (or CI equivalent) passes.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `file_lengths_within_limit` — script confirms no violations.

## Files touched

- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/diff.rs`
- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/session/tree.rs`
- `crates/runie-core/src/actors/config/ractor_config.rs`

## Notes

- Split by responsibility; do not just move code around.
