# Remove Nested Runtime from Subagent API

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`run_subagent` is a synchronous function that builds a new `current_thread` runtime and calls `block_on`. Although current callers wrap it in `spawn_blocking`, the API itself is dangerous if called from an async context (will panic). It also spins up a runtime per subagent.

## Acceptance Criteria

- [ ] Expose `run_subagent` as an `async fn`, or require an explicit runtime handle parameter.
- [ ] All callers are updated to `await` it or explicitly provide a handle.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `run_subagent_is_async` — function returns a future.

### Layer 2 — Event Handling
- [ ] `subagent_spawn_awaits_completion` — subagent completes without nested runtime.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-agent/src/subagent.rs`
- Call sites in `crates/runie-core/src/multi_agent.rs` and tests.

## Notes

This is R4 API hygiene; safe to defer if callers always use `spawn_blocking` today.
