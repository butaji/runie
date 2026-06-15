# Harden Test Verification and Fix dev.sh Smoke Mode

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

## Description

`scripts/verify-tests.sh` uses `set -e` without `pipefail`, so `cargo test 2>&1 | tee …` can hide failures. `dev.sh` has a `smoke` mode that calls `./scripts/smoke-tab-completion.sh` and `./scripts/smoke-turn-complete.sh`, which do not exist, and its usage line omits `smoke`.

## Acceptance Criteria

- [ ] `verify-tests.sh` uses `set -euo pipefail` and checks `cargo test` exit status explicitly.
- [ ] `dev.sh smoke` either runs a real smoke command or is removed.
- [ ] `dev.sh` usage text includes `smoke` if kept.
- [ ] `dev.sh` defaults to stable toolchain (nightly cranelift comment is stale).
- [ ] CI test-verification logic matches the script.

## Tests

### Layer 4 — Smoke
- [ ] A failing `cargo test` run causes `verify-tests.sh` to exit non-zero.
- [ ] `./dev.sh smoke` runs successfully in a tmux/screen session.
