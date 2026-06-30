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

- `.github/workflows/ci.yml` — added `cargo deny check` and `cargo machete` jobs
- `deny.toml` (new) — advisory checks, license allowlist, duplicate version warnings
- `Cargo.toml` — inherited license, workspace metadata

## Notes

- `cargo deny check` passes with only warnings (duplicates are all transitive)
- `cargo machete` passes with no unused dependencies
- Git2 unsound advisories are ignored pending `fff-search` update
- Duplicate versions are all transitive and would require upstream changes to resolve
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
