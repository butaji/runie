# Wire Actor/EventBus into runie-term

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: (none)
**Blocks**: event-bus-jsonl-persistence

## Description

The lightweight actor runtime decision is recorded in ADR 0017 and the minimal `Actor` trait + `EventBus` are implemented in `runie-core/src/actor.rs` and `bus.rs`. The terminal binary now wires the EventBus for cross-component communication.

## Acceptance Criteria

- [x] `runie-term/src/main.rs` creates one `EventBus<Event>` and spawns `SessionActor`.
- [x] Input reader and agent loop publish events to `EventBus` for `SessionActor` subscription.
- [x] `SessionActor` compiles, is declared in `lib.rs`, and persists durable events via `SessionStore`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `session_actor_filters_durable_events` — only durable events are appended to JSONL.

### Layer 2 — Event Handling
- [x] Events from input reader and agent loop are published to `EventBus`.

### Layer 3 — Rendering
N/A — existing render task continues to work via `watch` channel.

### Layer 4 — Smoke / Crash
- [x] `cargo build --release` succeeds.

## Files touched

- `crates/runie-core/src/session_actor.rs` — SessionActor implementation
- `crates/runie-core/src/lib.rs` — SessionActor re-export
- `crates/runie-core/src/bus.rs` — EventBus implementation
- `crates/runie-core/src/actor.rs` — Actor trait
- `crates/runie-term/src/main.rs` — EventBus wiring
- `crates/runie-term/Cargo.toml` — Added `dirs` dependency

## Notes

- `EventBus` is used for cross-component communication (SessionActor subscription).
- Main event loop still owns `AppState` directly for simplicity.
- Future work: Extract UiActor to own AppState and subscribe to bus.
- The wired EventBus also supports future actors such as the `FffIndexerActor` (`docs/adr/0023-fff-search-integration.md`).
