# Event Bus + JSONL Session Persistence

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

**Depends on**: actor-runtime-decision
**Blocks**: event-subenums, llm-event-normalization, context-compaction

## Description

Replace the current point-to-point `mpsc` channels in `runie-term/src/main.rs`
with a typed `EventBus<CoreEvent>` that all actors publish to and subscribe to.
Session persistence becomes a first-class `SessionActor` that writes durable
events to append-only JSONL files under `data_dir()` (and optionally a
project-local `.runie/` directory).

This implements the event-sourced model described in `docs/CONTEXT.md`:
a session is a persisted sequence of domain events that can be replayed into
actors.

## Acceptance Criteria

- [ ] `crates/runie-core/src/bus.rs` provides `EventBus<CoreEvent>` with:
  - `publish(event)` — non-blocking broadcast
  - `subscribe()` — returns `broadcast::Receiver<CoreEvent>`
  - `subscribe_replay(n)` — returns last N events + future events
  - `active_subscriber_count()` for diagnostics
- [ ] `CoreEvent` is split into durable vs transient:
  - `DurableCoreEvent` — persisted to JSONL (`MessageSent`, `ToolCalled`, `ToolResult`, `ModelSwitched`, `SessionRenamed`, etc.)
  - `TransientCoreEvent` — UI-only (`StreamingChunk`, `CursorBlink`, `Tick`)
- [ ] `crates/runie-core/src/session_store.rs` implements JSONL persistence:
  - File path: `data_dir/runie/sessions/<session_id>.jsonl`
  - Optional project-local mirror: `.runie/session-<id>.jsonl`
  - Each line is one JSON-serialized durable event
  - Advisory file lock (`fs2`) for multi-process safety
  - `load_events(session_id) -> Vec<DurableCoreEvent>` replays a session
- [ ] `SessionActor` subscribes to the bus, filters durable events, and appends
  them to the JSONL file atomically (write to temp + rename).
- [ ] `UiActor` subscribes to the bus, maintains `AppState`, and sends snapshots
  to the render task via the existing `watch` channel.
- [ ] `runie-term/src/main.rs` is rewired to:
  - Create one `EventBus<CoreEvent>`
  - Spawn `InputActor`, `AgentActor`, `ConfigActor`, `SessionActor`, `UiActor`
  - Remove the manual `input_rx`/`agent_rx` select in favor of the bus
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `session_store_appends_and_replays_events` — write 3 durable events,
  reload, assert order and content.
- [ ] `session_store_atomic_write_survives_crash` — temp+rename prevents
  partial writes.
- [ ] `event_bus_filters_durable_events` — `SessionActor` only writes durable
  events, transient ones are ignored.
- [ ] `jsonl_line_is_valid_json` — every persisted line deserializes to
  `DurableCoreEvent`.

### Layer 2 — Event Handling
- [ ] `published_event_reaches_all_subscribers` — two subscribers both receive
  the same event.
- [ ] `late_subscriber_gets_replay` — `subscribe_replay(5)` returns the last 5
  published events.

### Layer 3 — Rendering
- [ ] `ui_actor_snapshot_after_events` — feed a sequence of events into
  `UiActor` and assert the produced `Snapshot` contains the expected messages.

### Layer 4 — Smoke
- [ ] Run the binary, send a message, kill it, resume the session, and verify
  the conversation is restored from JSONL.

## Notes

**Why JSONL:**
- Human-readable, append-only, easy to inspect and debug.
- No SQLite dependency (explicit requirement).
- Each event is independently parseable; corruption in one line does not lose
  the whole session.

**Durable event design:**
Use a tagged enum so old events can be migrated:
```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "event", version = "v1")]
pub enum DurableCoreEvent {
    MessageSent { id, role, content, timestamp },
    ToolCalled { id, name, input },
    ToolResult { id, output, success },
    ModelSwitched { provider, model },
    SessionRenamed { name },
}
```

**Files touched:**
- `crates/runie-core/src/bus.rs`
- `crates/runie-core/src/session_store.rs` (new)
- `crates/runie-core/src/event.rs` (split into durable/transient)
- `crates/runie-term/src/main.rs`
- `crates/runie-core/src/state.rs` / `model.rs` (if needed for actor-owned state)

**Out of scope:**
- Full event sourcing with snapshots as load accelerators (can be added later).
- Migration from old `session.rs` JSON format (tracked separately if needed).
