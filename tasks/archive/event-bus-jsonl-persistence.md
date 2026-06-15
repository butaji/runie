# Event Bus + JSONL Session Persistence

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P0

**Depends on**: adopt-or-remove-actor-framework
**Blocks**: event-subenums, llm-event-normalization, tool-registry-trait, context-compaction, streaming-buffer-tail-split, session-list-summaries

## Description

`EventBus<CoreEvent>` in `runie-core/src/bus.rs` and `DurableCoreEvent` persistence in `crates/runie-core/src/session_store.rs` are implemented. SessionActor runs in runie-term and persists durable events to JSONL.

## Acceptance Criteria

- [x] `SessionStore::append` uses temp-file + rename for crash-safe atomic writes.
- [x] `SessionActor` compiles, is declared in `lib.rs`, and filters durable events correctly.
- [x] `SessionStore` exposes `update_index`/`SessionMeta` or the metadata index is removed from `SessionActor`.
- [x] `EventBus` and `DurableCoreEvent` are implemented and tested.
- [x] SessionActor runs in runie-term, persisting durable events to JSONL.
- [x] Old `/save` and `/load` commands continue to work with JSON format (for backward compatibility).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `session_store_appends_and_replays_events` — write durable events, reload, assert order.
- [x] `session_store_atomic_write_survives_crash` — temp+rename prevents partial writes.
- [x] `session_store_empty_when_no_file` — missing session returns empty event list.

### Layer 2 — Event Handling
- [x] `event_bus_filters_durable_events` — `SessionActor` only persists durable events.

### Layer 3 — Rendering
N/A — persistence has no direct rendering change.

### Layer 4 — Smoke / Crash
- [x] `cargo build --release` succeeds.

## Files touched

- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/session_actor.rs`
- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/bus.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-term/src/main.rs`

## Notes

- JSONL is the new primary persistence format for durable events (messages, tools, model switches).
- JSON format is kept for `/save` and `/load` commands for backward compatibility.
- Future: Migrate `/save`/`/load` to use JSONL directly (requires extending DurableCoreEvent).
