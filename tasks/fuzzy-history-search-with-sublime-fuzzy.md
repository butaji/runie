# Fuzzy history search with sublime-fuzzy

## Status

`done`

## Context

`crates/runie-core/src/input_history.rs:112-141` used hand-written prefix and substring search loops for history lookup. `sublime_fuzzy` is already a workspace dependency.

## Goal

Replace substring-based history search with fuzzy scoring via `sublime_fuzzy`, and surface the best matches first while keeping exact substring matches highly ranked.

## Changes Made

**`crates/runie-core/src/input_history.rs`:**

- Added `fuzzy_entry_score()` helper: scores an entry against a query using the same priority tiers as `dialog/score.rs`:
  - Exact prefix: 10 000+ (shorter = better)
  - Exact substring: 5 000+ (shorter = better)
  - Fuzzy via `sublime_fuzzy::best_match`: raw score (0-9 999)
- Replaced `search_history` with a scored, sorted implementation:
  - Scores all entries using `fuzzy_entry_score`
  - Sorts by score descending, then by recency descending (most recent first)
  - Returns all matching entries, ranked
- `filter_history` is unchanged: Up/Down prefix navigation must remain exact

## Acceptance Criteria

- [x] Replace `search_history` with fuzzy matching.
- [x] Keep exact substring matches highly ranked (prefix 10k+ > substring 5k+ > fuzzy 0-4k).
- [x] Ensure performance is acceptable for large history files (O(n) single pass).
- [x] History navigation (`Up`/`Down`, `/history`) works as before.

## Tests

- **Layer 1 — State/Logic:** `search_history_fuzzy_finds_typos`, `search_history_prefix_ranked_above_substring`, `search_history_exact_substring_ranked_above_fuzzy`, `search_history_case_insensitive`, `search_history_empty_query_returns_all`, `search_history_no_match_returns_empty`.
- **Layer 2 — Event Handling:** N/A (search_history is a pure utility; event handling unchanged).
- **Layer 3 — Rendering:** N/A (search_history affects result ordering, not rendering).
- **Layer 4 — E2E:** Covered by existing history navigation tests (`up_arrow_recalls_previous_input`, etc.).

## Files touched

- `crates/runie-core/src/input_history.rs` — added `fuzzy_entry_score`, rewrote `search_history`

## Completion Validation

- [x] **Unit tests** — `cargo test --package runie-core input_history` passes.
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — N/A (search_history is a utility function; `filter_history` for Up/Down unchanged).
