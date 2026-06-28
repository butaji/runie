# Remove `sleep()` from automatic tests

**Status**: todo
**Milestone**: R5
**Category**: Test harness
**Priority**: P2

**Depends on**: dedupe-turn-queue-delivery-logic
**Blocks**: none

## Description

`crates/runie-core/src/actors/session/tests.rs` uses `tokio::time::sleep` in five places. AGENTS.md explicitly forbids artificial delays in automatic tests. Replace sleeps with deterministic event/channel waits or instant test fixtures.

## Acceptance Criteria

- [ ] Remove all `tokio::time::sleep` calls from `crates/runie-core/src/actors/session/tests.rs`.
- [ ] Replace them with `tokio::sync::oneshot`/`notify` waits or pre-seeded state.
- [ ] Verify no other automatic tests contain `sleep` (excluding harness polling deadlines, which should be documented).
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 2 — Event Handling
- [ ] `session_actor_test_without_sleep` — session actor tests pass without delays.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/session/tests.rs`
- `crates/runie-testing/src/timeout.rs` (review harness deadlines)

## Notes

- `runie-testing/src/runner.rs` and `timeout.rs` contain short harness polling loops; those are acceptable if documented but should be reviewed.
- This task depends on a clean turn queue because tests may currently rely on timing to observe async side effects.
