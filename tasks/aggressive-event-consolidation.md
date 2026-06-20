# Aggressive event module consolidation (22 files → ~4)

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: simplify-event-vocabulary
**Blocks**: simplify-event-module-layout

## Description

The `crates/runie-core/src/event/` module is 22 files for a single 109-variant `Event` enum:

| File group | Count | Content |
|-------------|-------|---------|
| `variants.rs` + `variants/{constructors,to_durable}.rs` | 3 | Enum + impls |
| `variants_tests.rs` | 1 | Tests (outside the dir) |
| Per-domain group files (`login_flow`, `session`, `io`, `control`, `scroll`, `config`, `command`, `dialog`, `dialog_display`, `system`, `durable`, `agent`, `input`, `level`, `names`, `model_config`, `edit`) | 16 | Sub-enums / aliases |
| `mod.rs` | 1 | Module root |
| `event/login_flow.rs` alias shim | included above | 1-line `pub type` |

`simplify-event-module-layout` collapses the 5 alias shim files and moves `variants_tests.rs` into the dir (22 → ~15). `simplify-event-vocabulary` targets enum nesting. This task goes further: collapse the 16 per-domain group files into the variants module. The flat `Event` enum does not need 16 companion files — the variants, their constructors, durability, and names should live in ~4 files total.

## Acceptance Criteria

- [ ] `event/` directory reduced to ~4 files: `mod.rs`, `variants.rs` (enum + constructors + to_durable), `durable.rs` (durable/transient taxonomy), `names.rs` (event name strings).
- [ ] The 16 per-domain group files (`event/login_flow.rs`, `event/session.rs`, `event/io.rs`, `event/control.rs`, `event/scroll.rs`, `event/config.rs`, `event/command.rs`, `event/dialog.rs`, `event/dialog_display.rs`, `event/system.rs`, `event/durable.rs`, `event/agent.rs`, `event/input.rs`, `event/level.rs`, `event/model_config.rs`, `event/edit.rs`) deleted or merged.
- [ ] `event/variants/` dir flattened into `event/variants.rs` (if under 500 LOC) or kept as `variants/{mod,constructors,to_durable,tests}.rs` (4 files).
- [ ] All `use runie_core::event::{...}` imports still resolve via `event/mod.rs` re-exports.
- [ ] `arch_test_event_enum_variant_budget` still passes (baseline 109, budget 120).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_size_test_still_passes` — existing `event_size_reduced` test green after consolidation.
- [ ] `event_variant_count_unchanged` — 109 variants before and after.
- [ ] `durable_classification_unchanged` — `to_durable` produces the same durable/transient partition.

### Layer 2 — Event Handling
- [ ] `all_event_handlers_resolve_after_collapse` — every `update/` handler that matched on an event variant still compiles and routes correctly.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms no broken imports.

## Files touched

- `crates/runie-core/src/event/*.rs` — delete/merge 16 group files
- `crates/runie-core/src/event/mod.rs` — update module declarations + re-exports
- `crates/runie-core/src/event/variants.rs` — absorb constructors + to_durable if merging
- `crates/runie-core/src/event/variants/` — flatten or reduce to 4 files
- All files importing from deleted `event/<domain>.rs` files (grep-driven; re-exports cover most)

## Notes

Supersedes `simplify-event-module-layout` (which is less aggressive — keeps the per-domain files). Depends on `simplify-event-vocabulary` so the enum nesting stabilizes first. The 40-line function limit means large `match` blocks in constructors/to_durable may need to stay split by domain — if so, keep `variants/constructors.rs` split by domain as `constructors/{agent,session,io,dialog}.rs` but at the impl level, not the enum level. The goal is fewer files at the `event/` root, not fewer match arms.
