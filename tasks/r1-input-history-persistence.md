# Input history persistence and search

**Status**: done
**Milestone**: R1
**Category**: Input & Commands

## Description

Persist input history across sessions and add search/filter capability.

## Acceptance Criteria

- [x] Save history to `~/.runie/history.jsonl`
- [x] Load history on startup
- [x] `/history` command to show recent history
- [x] Filter by prefix match (via filter_history function)

## Implementation

### Files
- `crates/runie-core/src/input_history.rs` — History persistence and search module
- `crates/runie-core/src/update/input.rs` — Updated submit() to use add_to_input_history()
- `crates/runie-core/src/update/slash.rs` — Added /history command
- `crates/runie-core/src/lib.rs` — Exported input_history module

### Architecture

1. **History file**: `~/.runie/history.jsonl` - one JSON string per line
2. **Load on startup**: `AppState::load_input_history()` reads from file
3. **Save on submit**: `AppState::add_to_input_history()` saves after each command
4. **Search**: `filter_history()` and `search_history()` for prefix/substring matching

### Key Functions
- `load_history()` — Read entries from history file
- `save_history()` — Write all entries to history file
- `filter_history(entries, prefix)` — Filter by prefix (case-insensitive)
- `search_history(entries, query)` — Substring search, reverse chronological order
- `AppState::add_to_input_history()` — Add entry and persist

## Tests

### Layer 1 — State/Logic
- [x] `history_save_load_roundtrip` — JSON serialization of history entries
- [x] `filter_history_prefix_match` — Prefix filtering (case-insensitive)
- [x] `search_history_substring` — Substring search
- [x] `filter_history_empty_input` — Empty prefix returns all
- [x] `search_history_empty_query` — Empty query returns all

### Layer 2 — Event Handling
- [x] History navigation via Up/Down arrows (via existing HistoryPrev/HistoryNext events)
- [x] `/history` command shows recent history

## Notes

- Deferred from `mvp-input-history` which covers navigation only
