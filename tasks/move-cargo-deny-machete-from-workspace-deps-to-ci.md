# Move `cargo-deny` and `cargo-machete` from workspace deps to CI

**Status**: done
**Milestone**: R6
**Category**: Build / CI
**Priority**: P2

**Depends on**: introduce-cargo-deny-and-cargo-machete-ci
**Blocks**: none

## Description

`cargo-deny` and `cargo-machete` are declared in `[workspace.dependencies]` but they are binary tools, not library dependencies. Move installation to `.github/actions/rust-setup/action.yml` (or a dedicated CI step) and remove them from `Cargo.toml`.

## Acceptance Criteria

- [x] Remove `cargo-deny` and `cargo-machete` from `Cargo.toml` `[workspace.dependencies]`.
- [x] Install them in CI (caching where possible).
- [x] Ensure CI jobs still pass.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `cargo_toml_has_no_binary_tools` — `cargo-deny` and `cargo-machete` are not in workspace deps.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `Cargo.toml`
- `.github/actions/rust-setup/action.yml`
- `.github/workflows/ci.yml`

## Notes

- This reduces lockfile churn and clarifies which deps are actually linked.
