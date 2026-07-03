# Fix CI gates on `dev`

**Status**: done
**Milestone**: R7
**Category**: Architecture / Testing
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

The `dev` branch currently fails the CI pipeline defined in `.github/workflows/ci.yml`. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo deny check` all produce errors. The branch cannot be merged to `main` until these gates are green.

## Acceptance Criteria

- [x] `cargo fmt --all -- --check` passes with no diffs.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `cargo deny check` passes (bans + advisories).
- [x] `./scripts/check-file-limits.sh` is either green or its failures are explicitly accepted and documented.
- [x] `cargo test --workspace` passes after the changes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] N/A — CI/tooling concern.

### Layer 2 — Event Handling
- [x] N/A — CI/tooling concern.

### Layer 3 — Rendering
- [x] N/A — CI/tooling concern.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — CI/tooling concern.

### Live Tmux Testing Session
- [x] Run a headless `runie print` or TUI smoke run after fixes to confirm the CLI still starts and exits cleanly.

## Files touched

- `crates/runie-core/src/config/mod.rs` — derive `Default` for `SandboxSection` instead of manual impl.
- `deny.toml` — add skips for unavoidable duplicate `darling` versions and address `quick-xml` RUSTSEC-2026-0194.
- Various files reformatted by `cargo fmt --all`.

## Notes

- The `quick-xml` advisory (RUSTSEC-2026-0194) affects `0.37.5` via `plist` → `syntect`. Remediation is to upgrade `quick-xml` to `>= 0.41.0`, likely by updating `syntect`/`plist`.
- Duplicate `darling` versions (`0.20.11`, `0.21.3`, `0.23.0`) come from `derive_builder`/`validator` vs `tui-popup` vs `ratatui`. Add documented skips in `deny.toml` if they cannot be unified.
