# Extract composite CI setup action

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: centralize-test-verification
**Blocks**: none

## Description

`.github/workflows/ci.yml` copy-pastes `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, and cargo cache steps across the `test`, `fmt`, `clippy`, and `e2e` jobs. Any toolchain or cache change must be edited in four places.

## Acceptance Criteria

- [ ] A composite action at `.github/actions/rust-setup/action.yml` (or similar) encapsulates checkout, toolchain, and cache.
- [ ] All CI jobs use the composite action.
- [ ] CI still passes.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `ci_yaml_valid` — `actionlint` or `yamlfmt` validates the workflow.

## Files touched

- `.github/workflows/ci.yml`
- New `.github/actions/rust-setup/action.yml`

## Notes

Use a matrix for jobs that differ only in the final command (`cargo test`, `cargo fmt`, `cargo clippy`) to further reduce duplication.
