# Session persistence across restarts

**Status**: done

**Milestone**: MVP

**Category**: Sessions

## Description

Sessions persist across application restarts via JSONL event log.

## Acceptance Criteria

- [x] Auto-save on events — `record_event()` appends to JSONL
- [x] Resume from last state — `resume_session()` reopens session in append mode
- [x] Handle concurrent access — File system handles concurrent access natively

## Implementation

### Files

- `crates/runie-core/src/session_manager/mod.rs` — Actor and convenience functions
- `crates/runie-core/src/session_manager/state.rs` — SessionState actor state
- `crates/runie-core/src/session_manager/commands.rs` — SessionCmd/Response types
- `crates/runie-core/src/session_jsonl.rs` — JSONL serialization

### Architecture

1. **SessionState actor** subscribes to domain events
2. `record_event()` appends events to JSONL via `JsonlWriter`
3. `flush()` ensures writes hit disk (called periodically)
4. `close_session()` flushes and closes on shutdown
5. `resume_session()` reopens existing session in append mode

## Tests

### Layer 1 — State/Logic (session_manager)
- [x] `test_session_state_start_close` — Start/close session lifecycle
- [x] `test_session_state_record_events` — Event recording and pending queue
- [x] `test_session_state_snapshot_timing` — Snapshot interval logic
- [x] `test_session_roundtrip_via_jsonl` — JSONL serialization roundtrip
- [x] `test_delete_session` — Session deletion
- [x] `test_session_state_resume` — Resume and append to existing session
- [x] `test_convenience_session_path` — Path generation
- [x] `test_default_state` — Default state initialization

### Layer 1 — JSONL (session_jsonl)
- [x] `roundtrip_single_session` — Basic serialization roundtrip
- [x] `empty_events_list` — Empty event list handling
- [x] `append_writes_to_existing_file` — Append mode
- [x] `read_event_ignores_blank_lines` — Blank line handling
- [x] `line_numbers_increment` — Line number tracking
- [x] All DomainEvent variants serde roundtrip (13 variants)
- [x] SessionMeta serde roundtrip

All 9 session_manager tests and JSONL tests pass.
