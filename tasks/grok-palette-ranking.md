# Command Palette Ranking

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie's command palette filters by name/description but sorts by category/name.
Add Grok-style ranking: fuzzy match score boosted by recency and usage count,
all in-memory.

## Acceptance Criteria

- [ ] Palette items are ranked by `fuzzy_score(query, item) * recency_boost *
  usage_boost`.
- [ ] Recency and usage are stored only in memory (per the grilling decision).
- [ ] Recent invocations get a small boost.
- [ ] Frequently used commands get a small boost.
- [ ] Empty query still groups by category but shows recently used first.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn frequently_used_command_ranks_higher() {
    let mut state = AppState::default();
    state.record_command_usage("compact");
    state.record_command_usage("compact");
    let ranked = state.rank_palette("com");
    assert_eq!(ranked[0].name, "compact");
}

#[test]
fn recent_command_gets_recency_boost() {
    let mut state = AppState::default();
    state.record_command_usage("theme");
    state.record_command_usage("model");
    let ranked = state.rank_palette("");
    assert_eq!(ranked[0].name, "model");
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn invoking_command_records_usage() {
    let mut state = AppState::default();
    state.handle_slash("/compact");
    assert!(state.command_usage.contains_key("compact"));
}
```

## Files touched

- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/model/state.rs` (usage/recency fields)
- `crates/runie-core/src/update/dispatch.rs` (record on invoke)

## Out of scope

- Persisting usage across sessions.
- Fuzzy matcher replacement (use existing `crate::fuzzy`).
