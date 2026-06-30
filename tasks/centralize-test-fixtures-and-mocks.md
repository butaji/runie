# Centralize test fixtures and mocks in `runie-testing`

**Status**: done
**Note**: Verified 2026-06-29 — fixtures, MockToolSkill, ReplayProvider, and capture_events centralized in runie-testing.
**Milestone**: R1
**Category**: Testing
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

MiniMax SSE fixtures and mock helpers are duplicated across `runie-agent` and `runie-provider` integration tests. `runie-testing` already exists to share test infrastructure; move the reusable pieces there so provider and agent replay suites stop drifting.

## Acceptance Criteria

- [x] Move the MiniMax fixture loader (`fixture(name)`) into `runie-testing::fixtures::minimax`.
- [x] Move a shared `MockToolSkill` into `runie-testing::mock_tool_skill`.
- [x] Move a shared `ReplayProvider` and `capture_events()` helper into `runie-testing`.
- [x] Delete the duplicated copies in `runie-agent/tests/minimax_turn.rs`, `runie-provider/tests/minimax_replay.rs`, and `runie-agent/src/tests/turn.rs`.
- [x] Existing replay tests compile and pass using the centralized helpers.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `minimax_agent_replay_uses_shared_fixture` — `runie-agent` MiniMax turn test uses `runie_testing::fixtures::minimax`.
- [x] `minimax_provider_replay_uses_shared_fixture` — `runie-provider` replay test uses the same fixture loader.
- [x] `mock_tool_skill_shared` — both crates use `runie_testing::mock_tool_skill`.

### Layer 1 — State/Logic
- [x] `capture_events_helper_works` — centralized `capture_events` records the expected event sequence.

## Files touched

- `crates/runie-testing/src/lib.rs`
- `crates/runie-testing/src/fixtures/minimax.rs` (new or extend)
- `crates/runie-testing/src/mock_tool_skill.rs` (new)
- `crates/runie-testing/src/replay_provider.rs` (new)
- `crates/runie-agent/tests/minimax_turn.rs`
- `crates/runie-provider/tests/minimax_replay.rs`
- `crates/runie-agent/src/tests/turn.rs`

## Notes

- The fixtures themselves were already centralized in a prior task; this task focuses on the *helper code* around them.
- Keep helpers generic enough for future provider replay tests, not just MiniMax.
- Also consolidate duplicated `ENV_LOCK` mutexes and temp-dir/config-loading patterns from `runie-core/src/tests/support.rs` and `runie-provider/src/tests.rs` into `runie-testing`.
- `scripts/tmux-test.sh` is a shell/tmux test that violates AGENTS.md; any coverage it provides should be ported to Ratatui `TestBackend` tests in `runie-tui/src/tests/`.
- Rejected: leave duplication because the tests are in different crates — `runie-testing` exists precisely to prevent this.
