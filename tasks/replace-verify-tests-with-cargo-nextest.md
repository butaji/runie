# Replace verify-tests.sh with cargo-nextest

## Status

`todo`

## Context

`scripts/verify-tests.sh` uses `grep`/`awk` on `cargo test --list` output and checks `test result:` lines. It sets `RUST_TEST_TIMEOUT`, but `cargo test` does not honor it, so hanging tests are never killed.

## Goal

Replace the script with `cargo-nextest` configured via `.config/nextest.toml`, providing real per-test timeouts, retries, and groups. Update `justfile`/recipes accordingly.

## Acceptance Criteria

- [ ] Add `cargo-nextest` configuration with slow-timeout and per-test-group overrides.
- [ ] Delete `scripts/verify-tests.sh` (or reduce to a tiny wrapper).
- [ ] Ensure doctests are still run (`cargo test --doc`) because nextest skips them.
- [ ] CI uses `cargo nextest run`.
- [ ] All existing tests pass under nextest.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo nextest run --workspace` passes locally and in CI.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
