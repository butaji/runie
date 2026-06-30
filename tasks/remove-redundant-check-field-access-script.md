# Remove redundant `check-field-access.sh`

**Status**: done
**Note**: Verified 2026-06-29 — `scripts/check-field-access.sh` does not exist.
**Milestone**: R6
**Category**: Build / CI
**Priority**: P3

**Depends on**: replace-build-linter-with-clippy-ci
**Blocks**: none

## Description

`scripts/check-field-access.sh` duplicates the AppState field-access lint from `build.rs`. It also uses a PCRE negative lookahead that `ripgrep`'s default Rust regex engine does not support, so it likely does not run as intended. Remove it when the Clippy/CI linter replacement lands.

## Acceptance Criteria

- [x] Delete `scripts/check-field-access.sh`.
- [x] Remove any CI/recipe references.
- [x] Ensure the Clippy/CI replacement covers the same check.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `scripts/check-field-access.sh`
- `justfile`
- `.github/workflows/ci.yml`

## Notes

- Coordinate with `replace-build-linter-with-clippy-ci.md`.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
