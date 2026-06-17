# Merge SettingsRowKind and SettingValue Enums

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Two nearly identical enums in `dialog/builders.rs` and `settings.rs` represent the same concept — one for building settings rows, one for stored settings values. They have nearly identical variants and could be unified.

## Acceptance Criteria

- [ ] Analyze both enum variants and determine if they can merge
- [ ] If mergeable: consolidate into single enum, update all call sites
- [ ] If not mergeable: document why and close as wontfix
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] Settings tests pass after refactor

### Layer 2 — Event Handling
- [ ] Settings dialog tests pass

### Layer 3 — Rendering
- [ ] Settings UI tests pass

### Layer 4 — Smoke / Crash
- [ ] N/A

## Files touched

- `crates/runie-core/src/dialog/builders.rs`
- `crates/runie-core/src/commands/dsl/handlers/settings.rs`

## Notes

Low effort change with high clarity impact — having two enums for the same concept is confusing.
