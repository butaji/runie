# Track and cancel in-flight agent turn JoinHandle

## Status

`done`

## Context

`crates/runie-agent/src/actor.rs:137-164` spawned the agent turn as a detached `tokio::task` and discarded the `JoinHandle`. A second `Run` while one is in flight left the previous turn running with no cancellation or await path.

## Changes

- Added `current_turn_handle: Option<tokio::task::JoinHandle<()>>` to `AgentActorState`
- Added overlap check at start of `run_turn`: rejects new runs with error event if turn is in flight
- Store JoinHandle after spawning
- Updated `Abort` handler to abort and await the old handle
- Handle is properly cleaned up on abort

## Acceptance Criteria

- [x] Store `Option<JoinHandle<()>>` in actor state.
- [x] Reject or queue a new `Run` while one is in flight.
- [x] Cancel/await the handle on `Abort` and actor shutdown.
- [x] Surface turn errors instead of dropping them (already implemented).

## Tests

- **Layer 4 — E2E:** All workspace tests pass.
