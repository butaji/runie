# Fix dev automation recipes

**Status**: done
**Milestone**: R5
**Category**: Dev automation
**Priority**: P1

**Depends on**: none
**Blocks**: replace-build-linter-with-clippy-ci

## Description

`bacon.toml` and the `justfile` had broken or confusing recipes: `bacon.toml`'s `test` job ran the TUI binary instead of running tests, and `just lint-fix` passed contradictory Clippy flags (`-D warnings` with `-A clippy::all`). These have been fixed.

## Acceptance Criteria

- [x] Fix `bacon.toml` `test` job to run `cargo test -p runie-tui` (or the appropriate test command), not `cargo run`.
- [x] Fix `just lint-fix` to use `cargo clippy --fix --allow-dirty` (removed contradictory flags).
- [x] Remove the `check-skip` bacon job if the custom build linter is replaced; otherwise document why it is needed. (Kept: build linter still present in `crates/runie-core/build.rs`.)
- [x] `just test` and bacon `test` produce the expected behavior.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `bacon.toml`
- `justfile`

## Notes

- This task is about correctness of recipes, not adding new functionality.
- Coordinate with `replace-build-linter-with-clippy-ci.md` to remove the `check-skip` job when the build linter is replaced.
- The `check-skip` job is kept because `crates/runie-core/build.rs` still contains the custom linter. Once `replace-build-linter-with-clippy-ci` lands, this job can be removed.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
