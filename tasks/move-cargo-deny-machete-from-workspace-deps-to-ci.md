# Move `cargo-deny` and `cargo-machete` from workspace deps to CI

**Status**: todo
**Milestone**: R6
**Category": Build / CI
**Priority": P2

**Depends on**: introduce-cargo-deny-and-cargo-machete-ci
**Blocks**: none

## Description

`cargo-deny` and `cargo-machete` are declared in `[workspace.dependencies]` but they are binary tools, not library dependencies. Move installation to `.github/actions/rust-setup/action.yml` (or a dedicated CI step) and remove them from `Cargo.toml`.

## Acceptance Criteria

- [ ] Remove `cargo-deny` and `cargo-machete` from `Cargo.toml` `[workspace.dependencies]`.
- [ ] Install them in CI (caching where possible).
- [ ] Ensure CI jobs still pass.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo_toml_has_no_binary_tools` — `cargo-deny` and `cargo-machete` are not in workspace deps.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `Cargo.toml`
- `.github/actions/rust-setup/action.yml`
- `.github/workflows/ci.yml`

## Notes

- This reduces lockfile churn and clarifies which deps are actually linked.
