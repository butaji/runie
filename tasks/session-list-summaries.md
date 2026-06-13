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

- [ ] `crates/runie-core/src/session_store.rs` maintains a `sessions.jsonl`
  or `sessions.json` index in `data_dir/runie/` with metadata per session:
  - `id`, `display_name`, `created_at`, `updated_at`, `message_count`,
    `summary`, `is_starred`, `is_system`.
- [ ] `SessionActor` updates the index on every durable session event.
- [ ] `/sessions` slash command and `Ctrl+Shift+S` open the session list dialog.
- [ ] Session list dialog supports:
  - Fuzzy search by name/summary
  - Star/unstar
  - Rename
  - Delete (with confirmation)
  - Resume
- [ ] Auto-generated summary using a cheap model call when a session ends or
  reaches 10 messages.
- [ ] Named system sessions (e.g., "Scheduled Tasks") are pinned at the top.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `session_index_round_trips` — write metadata, reload, assert fields.
- [ ] `summary_generated_for_long_session` — mock provider returns summary.
- [ ] `starred_session_sorts_to_top` — sort order places starred first.

### Layer 2 — Event Handling
- [ ] `session_renamed_event_updates_index` — durable event updates metadata.

### Layer 3 — Rendering
- [ ] `session_list_renders_summary` — row shows summary text.
- [ ] `session_list_shows_starred_badge` — starred row has `★`.

## Notes

**Files touched:**
- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/commands/handlers/session.rs`
- `crates/runie-tui/src/popups.rs` or new `session_list.rs`

**Out of scope:**
- Session tree / branching UI (tracked by `r3-session-tree.md`).
- Visual timeline.
