# Split `ractor_turn.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-core/src/actors/turn/ractor_turn.rs` is 553 lines and mixes the handle, actor state, message handlers, and actor impl.

## Acceptance criteria

- Split into `state.rs`, `handlers.rs`, `actor.rs`.
- No module exceeds 500 lines.

## Tests

### Layer 2 — Event Handling
- `TurnMsg` handling still produces the same events.

### Layer 4 — Provider Replay / Mock-Tool E2E
- Multi-turn queue replay still works.
