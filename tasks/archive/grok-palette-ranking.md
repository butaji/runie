# Command Palette Ranking

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P1
**Completed in**: current

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie's command palette filtered by name/description but sorted by category/name.
Added Grok-style ranking: fuzzy match score boosted by recency and usage count,
all in-memory.

## Changes Made

### 1. `CommandUsage` tracking
- Added `CommandUsage { count: u32, last_used: f64 }` struct in `model/state.rs`
- Added `command_usage: HashMap<String, CommandUsage>` field to `AppState`
- Added `record_command_usage(&mut self, name: &str)` method that increments count
  and updates `last_used` timestamp
- Invoked in `handle_slash()` when a command is dispatched

### 2. Ranking function
- Added `rank_commands(&self, query: &str, limit: usize)` to `AppState`
- With query: fuzzy score × 100 + usage boost + recency boost
- Without query: usage count + recency boost sorted by score, then category/name
- `compute_ranking_score()` helper applies usage (count) and recency (decays over 5 min) boosts
- `open_command_palette()` now calls `rank_commands("", 100)` instead of `registry.list()`

### 3. Tests (in `commands/tests/usage.rs`)
- `frequently_used_command_ranks_higher` — `/compact` used twice outranks `model` for "com"
- `recent_command_gets_recency_boost` — most recently used command appears first for empty query
- `invoking_command_records_usage` — `/compact` invocation records usage
- `unknown_command_does_not_record_usage` — unknown commands skip recording
- `rank_commands_empty_query_returns_all` — empty query returns all commands
- `rank_commands_with_query_filters` — query "model" only matches model-related commands

## Acceptance Criteria

- [x] Palette items are ranked by `fuzzy_score(query, item) * recency_boost * usage_boost`
- [x] Recency and usage are stored only in memory (per the grilling decision)
- [x] Recent invocations get a small boost
- [x] Frequently used commands get a small boost
- [x] Empty query still groups by category but shows recently used first
- [x] `cargo test --workspace` succeeds

## Files touched

- `crates/runie-core/src/model/state.rs` (CommandUsage struct, record_command_usage, rank_commands)
- `crates/runie-core/src/commands/registry.rs` (record in handle_slash)
- `crates/runie-core/src/update/dialog.rs` (open_command_palette uses rank_commands)
- `crates/runie-core/src/commands/tests/usage.rs` (added 6 ranking tests)
