# Session List with Auto-Generated Summaries

**Status**: todo
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
- [ ] `/sessions` slash command and `Ctrl+Shift+S` open the session list dialog.
- [ ] Session list dialog supports:
  - Fuzzy search by name/summary
  - Star/unstar
  - Rename
  - Delete (with confirmation)
  - Resume
- [ ] Auto-generated summary using a cheap model call when a session ends or
  reaches 10 messages.
- [x] Named system sessions (e.g., "Scheduled Tasks") are pinned at the top.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds (with --test-threads=1 to avoid race).

## Tests

### Layer 1 — State/Logic
- [x] `session_index_round_trips` — write metadata, reload, assert fields.
- [ ] `summary_generated_for_long_session` — mock provider returns summary.
- [x] `starred_session_sorts_to_top` — sort order places starred first.
- [x] `session_list_builds_with_sections` — builds panel with System/Starred/Recent sections.
- [x] `session_list_empty_shows_message` — shows "no sessions" message.

### Layer 2 — Event Handling
- [x] `session_renamed_event_updates_index` — durable event updates metadata.

### Layer 3 — Rendering
- [ ] `session_list_renders_summary` — row shows summary text.
- [ ] `session_list_shows_starred_badge` — starred row has `★`.

## Files touched

- `crates/runie-core/src/session_index.rs`
- `crates/runie-core/src/dialog/builders.rs` — added `session_list` builder
- `crates/runie-core/src/event/control.rs` — added session control events

## Notes

- `session_list()` builder creates a filterable panel with session rows showing star, name, message count, and summary.
- `ControlEvent::SelectSession`, `StarSession`, `RenameSession`, `DeleteSession` added for session actions.
- UI integration (keyboard shortcut, slash command) still pending.
