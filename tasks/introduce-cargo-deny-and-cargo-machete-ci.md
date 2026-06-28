# Introduce `cargo-deny` and `cargo-machete` CI checks

**Status**: todo
**Milestone**: R5
**Category**: Build / CI
**Priority**: P2

**Depends on**: remove-unused-dependencies-and-normalize-workspace-deps
**Blocks**: none

## Description

The workspace has duplicate transitive dependencies, potential unused dependencies, and no automated audit. Add `cargo-deny` and `cargo-machete` to CI to keep the dependency graph lean and secure.

## Acceptance Criteria

- [ ] Add a `deny.toml` that bans duplicate versions of key crates, unmaintained crates, and security advisories (start with warnings for duplicates if too noisy).
- [ ] Add a CI job running `cargo deny check`.
- [ ] Add a CI job running `cargo machete` and fail on unused dependencies.
- [ ] Resolve any new failures or document exceptions.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo_deny_config_valid` — `cargo deny check` parses `deny.toml`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `.github/workflows/ci.yml`
- `deny.toml` (new)
- `Cargo.toml`
- `crates/*/Cargo.toml`

## Notes

- Run `cargo tree -d` to identify duplicates before configuring bans.
- `cargo-machete` may produce false positives for proc-macro crates or feature-only deps; use `--ignore` as needed.
