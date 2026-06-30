# Fix 500-line file-limit violations

**Status**: partial
**Milestone**: R7
**Category**: Build / CI
**Priority**: P1

**Depends on**: replace-build-linter-with-clippy-ci
**Blocks**: none

## Description

AGENTS.md enforces a 500-line limit per `.rs` file. Several production files exceed it. Split or refactor them to comply.

**Progress**: Fixed 3 of 6 files:
- `session/tree.rs`: 560 → 428 lines ✓
- `diff.rs`: 519 → 408 lines ✓
- `inspect.rs`: 521 → 474 lines ✓

**Remaining**:
- `ractor_config.rs`: 797 lines (285 lines of tests, production code needs splitting)
- `ractor_session_actor.rs`: 550 lines
- `config/mod.rs`: 550 lines
- `ui_actor.rs`: 563 lines (not in original list but violates limit)

## Acceptance Criteria

- [ ] All production `.rs` files are ≤ 500 lines.
- [ ] `scripts/check-file-limits.sh` (or CI equivalent) passes.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `file_lengths_within_limit` — script confirms no violations.

## Files touched

- `crates/runie-core/src/actors/config/ractor_config.rs` — 797 lines, needs production code refactoring
- `crates/runie-core/src/actors/session/ractor_session_actor.rs` — 550 lines
- `crates/runie-core/src/config/mod.rs` — 550 lines
- `crates/runie-tui/src/ui_actor.rs` — 563 lines

## Notes

- Split by responsibility; do not just move code around.
