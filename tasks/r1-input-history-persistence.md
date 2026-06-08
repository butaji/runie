# Input history persistence and search

**Status**: todo
**Milestone**: R1
**Category**: Input & Commands

## Description

Persist input history across sessions and add search/filter capability.

## Acceptance Criteria

- [ ] Save history to `~/.runie/history.jsonl`
- [ ] Load history on startup
- [ ] `/history` or Ctrl+R to search history
- [ ] Filter by prefix match

## Tests

- [ ] Layer 1 — `history_save_load_roundtrip` — write then read
- [ ] Layer 1 — `history_search_prefix` — filter by prefix
- [ ] Layer 2 — `ctrl_r_opens_history_search` — key binding

## Notes

- Deferred from `mvp-input-history` which covers navigation only
