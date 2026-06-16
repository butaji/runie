# Session List with Auto-Generated Summaries

**Status**: done (core implementation)
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: event-bus-jsonl-persistence

## Description

Runie can save/load sessions but has no rich session browser. Research from
Etienne (session drawer with 6-word LLM summaries, star/unstar, named system
sessions), gptme (branching logs), and Goose (SQLite session metadata) shows
that a session list with summaries and metadata greatly improves navigation.

## Acceptance Criteria

- [x] `crates/runie-core/src/session_index.rs` maintains a `sessions.json`
  index in `data_dir/runie/` with metadata per session:
  - `id`, `display_name`, `created_at`, `updated_at`, `message_count`,
    `summary`, `is_starred`, `is_system`.
- [x] `SessionActor` updates the index on every durable session event.
- [x] Session list dialog builder (`session_list()`) supports filtering by sections.
- [x] `ControlEvent::SelectSession`, `StarSession`, `RenameSession`, `DeleteSession` added.
- [x] Named system sessions (e.g., "Scheduled Tasks") are pinned at the top.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds (with --test-threads=1 to avoid race).

### Remaining (not yet implemented)
- `/sessions` slash command and `Ctrl+Shift+S` keyboard shortcut - UI wiring
- Auto-generated summary using LLM call
- Fuzzy search in dialog
- Star/unstar/rename/delete UI actions

## Tests

### Layer 1 — State/Logic
- [x] `session_index_round_trips` — write metadata, reload, assert fields.
- [x] `starred_session_sorts_to_top` — sort order places starred first.
- [x] `session_list_builds_with_sections` — builds panel with System/Starred/Recent sections.
- [x] `session_list_empty_shows_message` — shows "no sessions" message.

### Layer 2 — Event Handling
- [x] `session_renamed_event_updates_index` — durable event updates metadata.

### Layer 3 — Rendering
N/A - dialog builder tests cover structure.

## Files touched

- `crates/runie-core/src/session_index.rs`
- `crates/runie-core/src/dialog/builders.rs` — added `session_list` builder
- `crates/runie-core/src/event/control.rs` — added session control events

## Notes

- Core implementation complete: session index, dialog builder, control events.
- LLM summary generation and full UI integration deferred to future work.
