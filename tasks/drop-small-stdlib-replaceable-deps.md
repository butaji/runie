# Drop small stdlib-replaceable deps

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: drop-event-bus-replay-buffer
**Blocks**: none

## Description

Four small deps each have 1–3 use sites and a trivial stdlib replacement. Bundled into one task because each is small and they share the YAGNI / stdlib rationale:

| Dep | Sites | stdlib replacement |
|-----|-------|--------------------|
| `parking_lot` | `bus.rs`, `session_tree.rs`, `actors/fff_indexer/mod.rs` (3 sites) | `std::sync::Mutex` — critical sections are tiny, no perf concern; poisoning is acceptable (or use `parking_lot::type_state` pattern if needed, but std is enough) |
| `chrono` | `labels.rs::format_timestamp` (1 site) | UTC `HH:MM` math from `unix_secs` (the fallback already does this); or `time` crate if local time is required (smaller than chrono) |
| `nucleo-matcher` | `fuzzy.rs` (1 site) | ~20-LOC subsequence scorer for the short command/file lists runie actually fuzzy-matches; nucleo's SMPTE-grade fuzzy is overkill for <1000 items |
| `glob` | `permissions/rules.rs`, `permissions/mod.rs` (2 sites) | `std::fs::read_dir` recursion, or reuse `fff-search` (already a workspace dep) which is the file-discovery path runie already uses elsewhere |

`parking_lot` depends on `drop-event-bus-replay-buffer` (which removes the `bus.rs` use site). The other three are independent.

## Acceptance Criteria

- [ ] `parking_lot` removed: `rg "parking_lot" crates/` returns zero hits; `std::sync::Mutex` (or `RwLock`) used at the 3 sites.
- [ ] `chrono` removed: `rg "chrono" crates/` returns zero hits; `format_timestamp` produces `HH:MM` from unix seconds via stdlib math (or `time` crate if local time is justified).
- [ ] `nucleo-matcher` removed: `rg "nucleo_matcher" crates/` returns zero hits; `fuzzy.rs` uses a stdlib subsequence scorer; existing fuzzy tests stay green.
- [ ] `glob` removed: `rg "use glob\|glob::" crates/` returns zero hits; permission rules use `read_dir` recursion or `fff-search`.
- [ ] All four deps removed from `[workspace.dependencies]` and any crate `Cargo.toml` that declared them.
- [ ] `Cargo.lock` no longer pulls the four crates (and their transitive deps).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `format_timestamp_round_trips_known_times` — `format_timestamp(0.0)`, `format_timestamp(3661.0)` produce the expected `HH:MM` (UTC).
- [ ] `subsequence_scorer_ranks_prefix_above_middle` — a stdlib fuzzy scorer gives a higher score to "read" matching "read_file" than to "read" matching "thread_read".
- [ ] `permission_glob_matches_nested_path` — the `read_dir`-based permission matcher still matches `src/**/*.rs` style rules against nested files.

### Layer 2 — Event Handling
- [ ] `event_bus_pub_sub_with_std_mutex` — `EventBus` publish/subscribe round-trip still works after swapping `parking_lot::Mutex` for `std::sync::Mutex`.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_command_palette_fuzzy_still_ranks` — typing "md" in the command palette still surfaces "model" commands in the top results.
- [ ] `smoke_workspace_builds_after_dep_prune` — `cargo build --workspace` green; `Cargo.lock` compact.

## Files touched

- `crates/runie-core/src/bus.rs` (std `Mutex`)
- `crates/runie-core/src/session_tree.rs` (std `Mutex`)
- `crates/runie-core/src/actors/fff_indexer/mod.rs` (std `Mutex`)
- `crates/runie-core/src/labels.rs` (stdlib timestamp math)
- `crates/runie-core/src/fuzzy.rs` (stdlib subsequence scorer)
- `crates/runie-core/src/permissions/rules.rs` / `mod.rs` (`read_dir` recursion)
- `crates/runie-core/Cargo.toml`, `Cargo.toml` (remove 4 deps)

## Notes

If local-time `HH:MM` is required (user expectation), swap `chrono` for the smaller `time` crate instead of dropping to UTC — still a win over `chrono`. `nucleo-matcher` is the most debatable drop: it is small and fast, and the stdlib scorer is a regression in match quality. Weigh match-quality vs. dep count before deleting. Run after `drop-event-bus-replay-buffer` so `parking_lot` loses its `bus.rs` site first (otherwise that site must be migrated too).
