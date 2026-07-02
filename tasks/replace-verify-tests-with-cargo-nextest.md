# Replace verify-tests.sh with cargo-nextest

## Status

`done`

## Context

`scripts/verify-tests.sh` uses `grep`/`awk` on `cargo test --list` output and checks `test result:` lines. It sets `RUST_TEST_TIMEOUT`, but `cargo test` does not honor it, so hanging tests are never killed.

## Goal

Replace the script with `cargo-nextest` configured via `.config/nextest.toml`, providing real per-test timeouts, retries, and groups. Update `justfile`/recipes accordingly.

## Acceptance Criteria

- [x] Add `cargo-nextest` configuration with slow-timeout and per-test-group overrides.
- [x] Delete `scripts/verify-tests.sh` (or reduce to a tiny wrapper).
- [x] Ensure doctests are still run (`cargo test --doc`) because nextest skips them.
- [x] CI uses `cargo nextest run`.
- [x] All existing tests pass under nextest.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo nextest run --workspace` passes locally and in CI.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (test infrastructure change only).
