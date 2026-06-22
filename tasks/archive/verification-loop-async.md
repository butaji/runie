# Make Verification Loop Skill Asynchronous

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`VerificationLoopSkill::run_verification` executes the configured command synchronously inside `on_turn_end` and unwraps the result. A long verification command blocks the agent turn; `unwrap` can panic.

## Acceptance Criteria

- [x] Verification runs asynchronously.
- [x] Verification has a configurable timeout.
- [x] `unwrap` is removed; errors are converted to tool results.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `verification_timeout_returns_error` — timeout produces a graceful error.
- [x] `verification_failure_returns_result` — command failure does not panic.

### Layer 2 — Event Handling
- [x] `verification_emits_result_event` — async completion emits the result event.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/harness_skills/verification_loop.rs`

## Notes

Use `tokio::time::timeout` around an async command runner; reuse the consolidated bash implementation from `legacy-tool-enum-removal`.
