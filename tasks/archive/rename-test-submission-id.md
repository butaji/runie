# Rename test SubmissionId alias

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`runie-core/src/proto/op.rs` defines `struct SubmissionId(pub u64)` and `runie-testing/src/runner.rs` defines `pub type SubmissionId = String`. The same name with different semantics and types is confusing.

## Acceptance Criteria

- [ ] The test-runner alias is renamed (e.g., `TestSubmissionId`, `RunId`, or `TestRunId`).
- [ ] All test code referencing the alias is updated.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — rename only.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `runner_submission_id_still_string` — test runner still uses a string identifier.

## Files touched

- `crates/runie-testing/src/runner.rs`
- Any test files using `SubmissionId` from `runie-testing`.

## Notes

Trivial mechanical rename. Verify no other `SubmissionId` aliases exist after the change.
