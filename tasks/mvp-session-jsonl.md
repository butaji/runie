# Session JSONL format

**Status**: done

**Milestone**: MVP

**Category**: Sessions

## Description

Save/load sessions to JSONL files.

## Acceptance Criteria

- [x] JSONL serialization per event (DomainEvent → one JSON line per event)
- [x] File naming convention (session_name.jsonl)
- [x] Metadata header (type/version/name/provider/model/created_at/updated_at)
- [x] Streaming read/write for large sessions (JsonlReader/JsonlWriter)

## Tests

Implemented in `crates/runie-core/src/session_jsonl.rs`.

- [x] Layer 1 — roundtrip_single_session, empty_events_list, append_writes_to_existing_file, read_event_ignores_blank_lines, line_numbers_increment
- [x] Layer 1 — All DomainEvent variants serde roundtrip (13 variants)
- [x] Layer 1 — SessionMeta serde roundtrip
- [ ] Layer 2 — N/A (pure data transformation, no event handling)
- [ ] Layer 3 — N/A (no TUI rendering)
- [ ] Layer 4 — N/A (no async actor logic)
