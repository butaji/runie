# Drop small stdlib-replaceable deps

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: drop-event-bus-replay-buffer
**Blocks**: none

## Description

Four small deps each have 1–3 use sites and a trivial stdlib replacement. Bundled into one task because each is small and they share the YAGNI / stdlib rationale:

| Dep | Sites | stdlib replacement |
|-----|-------|--------------------|
| `parking_lot` | `bus.rs`, `session_tree.rs`, `actors/fff_indexer/mod.rs` (3 sites) | Already removed in prior work |
| `chrono` | `labels.rs::format_timestamp` (1 site) | UTC `HH:MM` math from `unix_secs`; timestamps now show UTC instead of local |
| `nucleo-matcher` | `fuzzy.rs` (1 site) | Simple subsequence scorer in `FuzzyMatcher`; same ranking for typical command names |
| `glob` | `permissions/rules.rs`, `permissions/mod.rs` (2 sites) | Custom `glob.rs` module with basic `*`, `**`, `?` pattern matching |

## Acceptance Criteria

- [x] `parking_lot` removed: `rg "parking_lot" crates/` returns zero hits (already removed).
- [x] `chrono` removed: `rg "chrono" crates/` returns zero hits; `format_timestamp` produces `HH:MM` from unix seconds via stdlib math.
- [x] `nucleo-matcher` removed: `rg "nucleo_matcher" crates/` returns zero hits; `fuzzy.rs` uses a stdlib subsequence scorer.
- [x] `glob` removed: `rg "use glob\|glob::" crates/` returns zero hits; permission rules use custom `glob.rs` module.
- [x] All four deps removed from `[workspace.dependencies]` and any crate `Cargo.toml` that declared them.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `format_timestamp_round_trips_known_times` — `format_timestamp(0.0)`, `format_timestamp(3661.0)` produce the expected `HH:MM` (UTC).
- [x] `fuzzy_matcher_scores_panel_items` — stdlib fuzzy scorer gives correct ranking for panel items.
- [x] `glob` tests cover all pattern types (`*`, `**`, `?`, exact match).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- Snapshot tests updated to reflect UTC timestamps.

### Layer 4 — Smoke / Crash
- [x] All tests pass (701+ tests).
- [x] `cargo build --workspace` green.

## Files touched

- `crates/runie-core/src/labels.rs` (removed chrono, simplified to UTC math)
- `crates/runie-core/src/fuzzy.rs` (replaced nucleo with simple scorer)
- `crates/runie-core/src/glob.rs` (new module for pattern matching)
- `crates/runie-core/src/permissions/mod.rs` (uses new glob module)
- `crates/runie-core/src/permissions/rules.rs` (uses new glob module)
- `crates/runie-core/src/lib.rs` (added glob module)
- `crates/runie-core/Cargo.toml` (removed chrono, nucleo-matcher, glob)
- `Cargo.toml` (removed nucleo-matcher from workspace deps)
- `crates/runie-tui/src/tests/snapshots/*.snap` (updated for UTC timestamps)

## Notes

- `parking_lot` was already removed in prior work.
- `chrono` removal changes timestamps from local time to UTC. Snapshot tests updated accordingly.
- `glob` replacement provides basic glob patterns sufficient for permission rules and sensitive path matching.
- `nucleo-matcher` replacement provides sufficient fuzzy matching for command palette and model selector (<1000 items).
