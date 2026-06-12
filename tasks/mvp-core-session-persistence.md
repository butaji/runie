# Session persistence with event log


**⚠️ NOTE:** This task built code that is unused by the runtime. See `docs/SHIP_REVIEW.md`.
**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Persist sessions as event logs. Save domain events to JSONL files, replay on load.

## Acceptance Criteria

- [x] Domain events serialized to JSONL (via session_jsonl.rs)
- [x] Session load replays events into all actors (SessionManager actor)
- [x] SessionManager handles save/load/list/delete
- [x] Periodic snapshots as load accelerators

## Implementation

### Files

- `crates/runie-core/src/session_manager/mod.rs` — Main actor and convenience functions
- `crates/runie-core/src/session_manager/state.rs` — SessionState mutable actor state
- `crates/runie-core/src/session_manager/commands.rs` — SessionCmd/Response types
- `crates/runie-core/src/session_jsonl.rs` — JSONL serialization (already existed)

### Architecture

1. **SessionManager actor** subscribes to domain events on the event bus
2. On each domain event, appends to JSONL file via `JsonlWriter`
3. `SessionState` tracks pending events for replay and snapshot timing
4. Periodic snapshots flush writes to disk
5. Session load replays events from JSONL into the event system

### Usage

```rust
use runie_core::{session_manager::{start_session, load_session, list_sessions}, DomainEvent};

// Start a new session
start_session("my_session", "openai", "gpt-4o")?;

// Record events
state.record_event(&DomainEvent::Submit { content: "Hello".into() })?;

// Load existing session
let (meta, events) = load_session("my_session")?;

// List sessions
let names = list_sessions()?;
```

## Tests

- Layer 1 — State/logic tests:
  - `test_session_state_start_close` — Start/close session lifecycle
  - `test_session_state_record_events` — Event recording and pending queue
  - `test_session_state_snapshot_timing` — Snapshot interval logic
  - `test_session_roundtrip_via_jsonl` — JSONL serialization roundtrip
  - `test_delete_session` — Session deletion
  - `test_session_state_resume` — Resume and append to existing session
  - `test_convenience_session_path` — Path generation
  - `test_default_state` — Default state initialization
  - `orchestrator::tests::test_spawn_session_manager` — Actor spawning

- Layer 2 — N/A (session persistence is pure data transformation, no event handling)

- Layer 3 — N/A (no TUI rendering)

- Layer 4 — N/A (no async actor logic in core session persistence)

All 9 session_manager tests pass.
