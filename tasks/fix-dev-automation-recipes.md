# Fix dev automation recipes

**Status**: todo
**Milestone**: R5
**Category**: Dev automation
**Priority**: P1

**Depends on**: none
**Blocks**: replace-build-linter-with-clippy-ci

## Description

`bacon.toml` and the `justfile` have broken or confusing recipes: `bacon.toml`'s `test` job actually runs the TUI binary, and `just lint-fix` passes contradictory Clippy flags. Fix them so developer tooling is reliable.

## Acceptance Criteria

- [ ] Fix `bacon.toml` `test` job to run `cargo test -p runie-tui` (or the appropriate test command), not `cargo run`.
- [ ] Fix `just lint-fix` to use `cargo clippy --fix --allow-dirty` or remove the recipe.
- [ ] Remove the `check-skip` bacon job if the custom build linter is replaced; otherwise document why it is needed.
- [ ] `just test` and bacon `test` produce the expected behavior.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] N/A.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `bacon.toml`
- `justfile`
- `dev.sh` (if circular delegation is resolved)

## Notes

- This task is about correctness of recipes, not adding new functionality.
- Coordinate with `replace-build-linter-with-clippy-ci.md` to remove the `check-skip` job.
