# Remove redundant `check-field-access.sh`

**Status**: done
**Milestone**: R6
**Category**: Build / CI
**Priority**: P3

**Depends on**: replace-build-linter-with-clippy-ci
**Blocks**: none

## Description

`scripts/check-field-access.sh` duplicates the AppState field-access lint from `build.rs`. It also uses a PCRE negative lookahead that `ripgrep`'s default Rust regex engine does not support, so it likely does not run as intended. Remove it when the Clippy/CI linter replacement lands.

## Acceptance Criteria

- [ ] Delete `scripts/check-field-access.sh`.
- [ ] Remove any CI/recipe references.
- [ ] Ensure the Clippy/CI replacement covers the same check.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `scripts/check-field-access.sh`
- `justfile`
- `.github/workflows/ci.yml`

## Notes

- Coordinate with `replace-build-linter-with-clippy-ci.md`.
