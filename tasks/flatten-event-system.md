# Flatten Event System

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: coalesce-update-modules

## Description

The `Event` type is currently split into 11 sub-enums living in separate
files (`event/input.rs`, `event/agent.rs`, etc.) plus a 400-line hand-written
`event/names.rs` string-to-event lookup table and 50 convenience constructors
in `event/variants.rs`. Adding one event requires touching the sub-enum,
`variants.rs`, `names.rs`, and the two-level dispatcher in
`update/mod.rs` → `update/dispatch.rs`.

This structure was introduced to reduce match-arm size, but the cure is worse
than the disease. A flat `Event` enum with generated name mapping is simpler
to understand and modify.

## Acceptance Criteria

- [ ] `Event` is a flat enum of all variants (or a single enum with a
generated `From`/`TryFrom` for sub-views if needed for dispatch clarity).
- [ ] `event/names.rs` is deleted; event-name mapping is generated via a
derive macro or `strum`.
- [ ] `update/mod.rs` and `update/dispatch.rs` are merged into a single
dispatcher.
- [ ] Convenience constructors on `Event` are either generated or removed in
favor of explicit construction.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_name_round_trip` — every event variant serializes to a stable
  string and back.
- [ ] `dispatcher_handles_all_variants` — the merged dispatcher has an arm
  for every `Event` variant.

### Layer 2 — Event Handling
- [ ] `keybinding_resolves_to_event` — a keybinding string resolves to the
  same `Event` as before.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/names.rs`
- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/event/*.rs` (sub-enums)
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/keybindings.rs`

## Notes

Supersedes `tasks/event-subenums.md`.

The current sub-enums can be flattened mechanically; the important change is
removing the hand-maintained `names.rs` table and the double dispatcher.
