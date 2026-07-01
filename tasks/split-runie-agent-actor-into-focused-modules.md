# Split `runie-agent/src/actor.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-agent/src/actor.rs` is 578 lines and mixes messages, actor impl, turn setup, factory, and leader integration.

## Acceptance criteria

- Split into `state.rs`, `turn.rs`, `factory.rs`.
- No module exceeds 500 lines.

## Tests

### Layer 2 — Event Handling
- `AgentMsg` handling still works.

### Layer 4 — Provider Replay / Mock-Tool E2E
- A replay turn completes through the refactored actor.
