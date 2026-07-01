# Externalize default theme TOML to resources

## Status

`done`

## Context

`crates/runie-tui/src/semantic_tokens.rs` (previously lines 80–161) embedded the canonical theme as a 160-line raw Rust string. It cannot be previewed/edited with standard TOML tooling.

## Goal

Move `DEFAULT_THEME_TOML` to `crates/runie-tui/resources/themes/runie.toml` and load it with `include_str!`. Remove the hand-maintained theme token list.

## Acceptance Criteria

- [x] Move theme TOML to resources.
- [x] Load it at startup.
- [x] Delete raw string and manual token list.
- [x] Theme rendering unchanged.

## Design Impact

No change to TUI element design or composition. Only theme loading changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that theme loads and parses.
- **Layer 2 — Event Handling:** Theme-loaded fact unchanged.
- **Layer 3 — Rendering:** `TestBackend` snapshots match.
- **Layer 4 — E2E:** Headless CLI loads theme.
- **Live tmux testing session (required):** TUI starts with the same theme colors.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
