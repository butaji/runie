# Introduce `cargo-deny` and `cargo-machete` CI checks

**Status**: done
**Milestone**: R5
**Category**: Build / CI
**Priority**: P2

**Depends on**: remove-unused-dependencies-and-normalize-workspace-deps
**Blocks**: none

## Description

The workspace had duplicate transitive dependencies, potential unused dependencies, and no automated audit. Added `cargo-deny` and `cargo-machete` to CI to keep the dependency graph lean and secure.

## Acceptance Criteria

- [x] Add a `deny.toml` that bans duplicate versions of key crates, unmaintained crates, and security advisories (start with warnings for duplicates if too noisy).
- [x] Add a CI job running `cargo deny check`.
- [x] Add a CI job running `cargo machete` and fail on unused dependencies.
- [x] Resolve any new failures or document exceptions.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `cargo_deny_config_valid` — `cargo deny check` parses `deny.toml`. (Verified manually; `cargo deny check` exits 0.)

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `.github/workflows/ci.yml`
- `deny.toml` (new)
- `Cargo.toml`
- `crates/runie-core/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-provider/Cargo.toml`
- `crates/runie-agent/Cargo.toml`

## Changes

- Added `deny.toml` with:
  - Advisory checks (warn on workspace unmaintained, ignore git2 unsound advisories pending fff-search update)
  - License allowlist covering all transitive dependencies
  - Duplicate version warnings (no bans yet, can be tightened later)
  - Skip tree for `fff-search` (external crate)
- Added CI jobs for `cargo deny check` and `cargo machete`
- Removed unused dependencies discovered by `cargo machete`:
  - `jsonschema` from `runie-core` (replaced by hand-written validator)
  - `anyhow`, `serde` from `runie-tui`
  - `backon`, `bytes`, `pin-project` from `runie-provider`
  - `rmcp` from `runie-agent` (not used in agent code)
- Added `license = "MIT OR Apache-2.0"` to workspace package and inherited by all crates

## Notes

- `cargo deny check` passes with only warnings (duplicates, which are transitive)
- `cargo machete` passes with no unused dependencies
- The duplicate versions are all transitive and would require upstream changes to resolve
- Git2 unsound advisories are ignored pending `fff-search` update
