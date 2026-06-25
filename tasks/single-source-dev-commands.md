# Single-source dev commands

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`cargo test --workspace`, `cargo clippy --workspace`, and `cargo fmt` appear in `README.md`, `.github/workflows/ci.yml`, `dev.sh`, and `bacon.toml`. These entry points drift out of sync.

## Acceptance Criteria

- [x] `justfile` recipes (e.g., `just test`, `just lint`, `just fmt`) become the canonical commands.
- [x] `dev.sh`, `README.md`, and `bacon.toml` point to `just` recipes where possible.
- [x] CI either uses the `just` recipes or the same underlying commands.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `just_test_runs_workspace_tests` — `just test` passes.

## Files touched

- `justfile`
- `dev.sh`
- `bacon.toml`
- `README.md`
- `.github/workflows/ci.yml`

## Notes

If `just` is not a project dependency, add it to dev setup docs or use a plain `scripts/dev.sh` wrapper that other files reference.
