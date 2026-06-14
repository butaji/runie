# Fix Clippy Errors and Test/CI Hygiene

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

The workspace currently fails strict clippy, has one failing unit test, and the CI
expected test counts are stale. The project also advertises 500-line/40-line lint limits
in `AGENTS.md`/`README.md` while `crates/runie-core/build.rs` enforces 2000/150/30.

## Acceptance Criteria

- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [x] `cargo test --workspace --lib` passes (the failure was caused by another
  test mutating `HOME`; fixed by making `auth::machine_key` respect
  `RUNIE_MACHINE_KEY`).
- [x] Update `EXPECTED_TOTAL` in `.github/workflows/ci.yml` and
  `scripts/verify-tests.sh` to the current 1,803 tests.
- [x] Reconcile the lint limit mismatch with **Option B**: update
  `AGENTS.md`/`README.md` to the actual 2000/150/30 limits and document the
  long-term targets.
- [x] All build and test verification scripts pass.

## Tests

### Layer 1 — State/Logic
- [x] `save_persists_pending_profile` passes with a stable `RUNIE_MACHINE_KEY`.

### Layer 2 — Event Handling
- [x] No event changes.

### Layer 3 — Rendering
- [x] No rendering changes.

### Layer 4 — Smoke
- [x] `scripts/verify-tests.sh` exits 0.

## Files touched

- `crates/runie-core/src/tests/*.rs` (clippy fixes)
- `crates/runie-core/src/update/tab_complete.rs`
- `crates/runie-core/src/state.rs`
- `crates/runie-core/src/commands/agents_manager.rs` (failing test)
- `crates/runie-core/build.rs` or `AGENTS.md`/`README.md` (lint limit reconciliation)
- `.github/workflows/ci.yml`
- `scripts/verify-tests.sh`

## Out of scope

- Functional refactors beyond the minimum needed to satisfy clippy.
- Rewriting tests that only need `#[allow(...)]` removals or borrow fixes.
