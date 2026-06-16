# Box Large Event Variants

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: flatten-event-enum
**Blocks**: (none)

## Description

Clippy warns that several enums are at least 288 bytes due to large embedded payloads (`PanelStack`, `String`, etc.): `Event`, `commands/dsl/flow.rs`, `dialog/builders.rs`, `dialog/item.rs`, `update/dialog/form.rs`. Large discriminants hurt cache locality.

## Acceptance Criteria

- [ ] Large enum variant payloads are boxed where ownership allows.
- [ ] Enum sizes are reduced below clippy's large-variant threshold.
- [ ] `cargo clippy --workspace` no longer warns on these enums.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_size_reduced` — `std::mem::size_of::<Event>()` is smaller than baseline.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/dialog/builders.rs`
- `crates/runie-core/src/dialog/item.rs`
- `crates/runie-core/src/update/dialog/form.rs`

## Notes

Measure before and after; do not box variants that are hot in tight loops without profiling.
