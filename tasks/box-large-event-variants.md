# Box Large Event Variants

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: flatten-event-enum
**Blocks**: (none)

## Description

Clippy warns that several enums are at least 288 bytes due to large embedded payloads (`PanelStack`, `String`, etc.): `Event`, `commands/dsl/flow.rs`, `dialog/builders.rs`, `dialog/item.rs`, `update/dialog/form.rs`. Large discriminants hurt cache locality.

## Acceptance Criteria

- [x] Large enum variant payloads are boxed where ownership allows.
- [x] Enum sizes are reduced below clippy's large-variant threshold.
- [x] `cargo clippy --workspace` no longer warns on these enums.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `event_size_reduced` — `std::mem::size_of::<Event>()` is smaller than baseline (288 → 104 bytes).

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/dialog/builders.rs`
- `crates/runie-core/src/dialog/item.rs`
- `crates/runie-core/src/update/dialog/form.rs`

## Notes

- `Event` large orchestrator payloads (`OrchestratorState`, `OrchestratorPlan`, `SubagentTask`) are now boxed.
- `CommandResult::OpenPanelStack` now holds `Box<PanelStack>`.
- Removed `#[allow(clippy::large_enum_variant)]` from `ItemAction`, `SettingsRowKind`, `FormAction`, and `CommandResult`.
- Final `Event` size: 104 bytes (baseline 288 bytes).
