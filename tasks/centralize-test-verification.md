# Centralize test-count verification

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P0

**Depends on**: none
**Blocks**: extract-ci-setup-action

## Description

Test-count verification logic is duplicated between `.github/workflows/ci.yml` and `scripts/verify-tests.sh`. The two copies are already out of sync: CI expects 1806 total tests while the script expects 2271. The same counting/grepping logic must not be maintained in two places.

## Acceptance Criteria

- [ ] `scripts/verify-tests.sh` becomes the single source of truth for test counting.
- [ ] `.github/workflows/ci.yml` calls `scripts/verify-tests.sh` instead of inlining the bash.
- [ ] The expected total count is the same in both entry points.
- [ ] `cargo test --workspace` followed by `./scripts/verify-tests.sh` passes locally.
- [ ] CI `test` job passes.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — build/script change.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `verify_tests_script_passes` — run `./scripts/verify-tests.sh` and assert exit 0.
- [ ] `ci_yaml_invokes_verify_script` — parse `.github/workflows/ci.yml` and assert the test job calls the script.

## Files touched

- `.github/workflows/ci.yml`
- `scripts/verify-tests.sh`

## Notes

If the script needs flags for CI vs local behavior (e.g., strict fail-fast), add them rather than duplicating logic.
