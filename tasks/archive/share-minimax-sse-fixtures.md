# Share duplicated MiniMax SSE fixtures

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Seven MiniMax SSE fixture files are byte-identical in `crates/runie-agent/tests/fixtures/minimax/*` and `crates/runie-provider/tests/fixtures/minimax/*`. Updates must be copied manually, which guarantees drift.

## Acceptance Criteria

- [ ] All 7 duplicated fixtures live in exactly one location.
- [ ] Both `runie-agent` and `runie-provider` tests load from the shared path.
- [ ] No byte-identical fixture pairs remain between the two crates.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — fixture move.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_m3_list_files_fixture_still_replayable` — provider replay test using the shared path.
- [ ] `minimax_m3_agent_turn_fixture_still_replayable` — agent turn test using the shared path.

## Files touched

- `crates/runie-agent/tests/fixtures/minimax/`
- `crates/runie-provider/tests/fixtures/minimax/`
- `crates/runie-testing/src/fixtures.rs` (possible shared home)
- Test files that reference the fixture paths.

## Notes

Preferred homes: `crates/runie-testing/src/fixtures/` (if both test suites can depend on it) or `crates/runie-provider/tests/fixtures/` (if agent tests can reach it via `CARGO_MANIFEST_DIR` relative paths). Verify binary inclusion if using `include_str!`.
