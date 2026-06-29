# Clean up `verify-tests.sh` and `just lint-fix`

**Status**: done
**Milestone**: R6
**Category**: Build / CI
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`scripts/verify-tests.sh` hardcodes `EXPECTED_TOTAL=2657`, which must be updated manually and is fragile. `just lint-fix` passes `-- -A clippy::all`, allowing all lints and therefore fixing nothing. Clean both up.

## Acceptance Criteria

- [ ] Remove the brittle `EXPECTED_TOTAL` exact-count assertion from `verify-tests.sh`; keep `MIN_TESTS` and failure/panic checks.
- [ ] Fix `just lint-fix` to use `cargo clippy --fix --allow-dirty --allow-staged -- -D warnings`, or remove the recipe.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `scripts/verify-tests.sh`
- `justfile`

## Notes

- These are dev-tooling quality-of-life fixes.
