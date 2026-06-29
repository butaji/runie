# Replace `scripts/tmux-test.sh` with Ratatui `TestBackend` tests

**Status**: done
**Milestone**: R5
**Category**: Test harness
**Priority**: P2

**Depends on**: unify-tui-render-test-helpers
**Blocks**: none

## Description

`scripts/tmux-test.sh` is a shell/tmux integration test with `sleep 5` and pane-text grepping. It violates AGENTS.md anti-patterns. Port its coverage to deterministic Rust tests using Ratatui `TestBackend`.

## Acceptance Criteria

- [ ] Identify the behaviors `scripts/tmux-test.sh` currently verifies.
- [ ] Implement equivalent `TestBackend` tests in `crates/runie-tui/src/tests/`.
- [ ] Delete `scripts/tmux-test.sh`.
- [ ] Remove or update any CI/recipe references to the script.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 3 — Rendering
- [ ] `tmux_smoke_rendered_in_buffer` — the startup/render behavior covered by the tmux test matches a `TestBackend` buffer.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_smoke_e2e_without_shell` — the same scenario runs through the mock-provider E2E harness.

## Files touched

- `scripts/tmux-test.sh` (delete)
- `crates/runie-tui/src/tests/mod.rs`
- `crates/runie-tui/src/tests/snapshot.rs`
- `justfile`
- `.github/workflows/ci.yml`

## Notes

- AGENTS.md forbids shell/tmux tests; this task brings the project into compliance.
- If the tmux test is already unmaintained/broken, deleting it without replacement is acceptable only after documenting the lost coverage.
