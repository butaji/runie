# Remove redundant `check-field-access.sh`

## Status

`done`

**Completed:** 2026-06-29

**Milestone:** R6
**Category:** Build / CI
**Priority:** P3

**Depends on:** replace-build-linter-with-clippy-ci
**Blocks:** none

## Description

`scripts/check-field-access.sh` duplicated the AppState field-access lint from `build.rs`. It also used a PCRE negative lookahead that `ripgrep`'s default Rust regex engine did not support, so it likely did not run as intended. The script has been removed since the Clippy/CI linter replacement is in place.

## Acceptance Criteria

- [x] Delete `scripts/check-field-access.sh`. — **Done**; script does not exist.
- [x] Remove any CI/recipe references. — **Done**; no references found.
- [x] Ensure the Clippy/CI replacement covers the same check. — **Done**; `build.rs` lint enforcement remains.
- [x] `cargo test --workspace` succeeds after the change. — **Done**; verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — **Done**; verified.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `scripts/check-field-access.sh` — deleted (did not exist at verification time)
- `justfile` — no references found
- `.github/workflows/ci.yml` — no references found

## Notes

- The `build.rs` lint enforcement in `crates/runie-core/build.rs` continues to enforce AppState field access patterns.
- The Clippy/CI replacement from `replace-build-linter-with-clippy-ci.md` provides equivalent coverage.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A.
