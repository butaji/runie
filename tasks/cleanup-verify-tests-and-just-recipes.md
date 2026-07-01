# Clean up `verify-tests.sh` and `just lint-fix`

**Status**: done
**Milestone**: R6
**Category**: Build / CI
**Priority**: P3
**Note**: verify-tests.sh uses MIN_TESTS; just lint-fix uses -D warnings.

**Depends on**: none
**Blocks**: none

## Description

`scripts/verify-tests.sh` hardcodes `EXPECTED_TOTAL=2657`, which must be updated manually and is fragile. `just lint-fix` passes `-- -A clippy::all`, allowing all lints and therefore fixing nothing. Clean both up.

## Acceptance Criteria

- [x] Remove the brittle `EXPECTED_TOTAL` exact-count assertion from `verify-tests.sh`; keep `MIN_TESTS` and failure/panic checks.
- [x] Fix `just lint-fix` to use `cargo clippy --fix --allow-dirty --allow-staged -- -D warnings`, or remove the recipe.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A. — Dev-tooling changes verified by `cargo check --workspace`.

## Files touched

- `scripts/verify-tests.sh`
- `justfile`

## Notes

- These are dev-tooling quality-of-life fixes.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
