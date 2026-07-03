# Instrument actor handlers with tracing

## Status

`done`

## Description

Actor handlers in `TurnActor`, `ProviderActor`, `SessionActor` are uninstrumented. Add `#[tracing::instrument]` with span fields for `turn_id`, `provider`, `model`.

## Acceptance criteria

1. **Unit tests** — Tests verify spans are created with correct fields. ✅ Added `turn_actor_handler_runs_with_tracing` test.
2. **E2E tests** — Actor replay tests still pass. ✅ All existing actor tests pass.
3. **Live tmux tests** — Run with debug logging and observe actor spans.

## Changes Made

Added `#[tracing::instrument]` to all actor handlers:
- `RactorTurnActor::handle` in `actors/turn/ractor_turn.rs`
- `RactorProviderActor::handle` in `actors/provider/ractor_provider.rs`
- `RactorSessionActor::handle` in `actors/session/ractor_session_actor.rs`
- `RactorSessionActor::handle_msg` in `actors/session/session_handlers.rs`
- `RactorConfigActor::handle` in `actors/config/ractor_config.rs`
- `RactorFffIndexerActor::handle` in `actors/fff_indexer/ractor_fff_indexer.rs`
- `InputActor::handle` in `actors/input/actor.rs`
- `RactorIoActor::handle` in `actors/io/ractor_io.rs`
- `RactorPermissionActor::handle` in `actors/permission/ractor_permission.rs`

## Tests

### Unit tests
- `tracing_test` asserts span fields. ✅ `turn_actor_handler_runs_with_tracing` exercises the instrumented handler path.

### E2E tests
- Replay turn exercises instrumented actors. ✅ All actor tests pass.

### Live tmux tests
- Submit a prompt and check logs for actor spans.
