# Flatten Event Enum

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P0

**Depends on**: (none)
**Blocks**: coalesce-update-modules

## Description

`Event` in `runie-core/src/event/variants.rs` is a 13-variant wrapper over sub-enums (`Input(InputEvent)`, `Agent(AgentEvent)`, …). `docs/SPEC.md` says R3 is flattening the event system, but the code has gone the opposite direction. Two-level matching complicates dispatch and `EVENT_NAMES` generation.

## Acceptance Criteria

- [ ] `Event` is a single flat enum with all variants at the top level.
- [ ] Sub-enum wrapper layer is removed or reduced to type aliases.
- [ ] `update/mod.rs` dispatcher matches variants directly.
- [ ] `EVENT_NAMES` is derived or generated without the wrapper indirection.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_name_round_trip` — every variant serializes to/from its name.
- [ ] `dispatch_exhaustive` — every variant has a dispatch arm.

### Layer 2 — Event Handling
- [ ] `all_input_events_dispatch` — input events still reach handlers.
- [ ] `all_agent_events_dispatch` — agent events still reach handlers.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/event/names.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/*.rs`

## Notes

`flatten-event-system.md` was previously marked done but only renamed/refactored the naming layer; the enum is still nested.
