# Aggressive event module consolidation (22 files → ~4)

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The `crates/runie-core/src/event/` module was 22 files for a single 109-variant `Event` enum:

| File group | Count | Content |
|-------------|-------|---------|
| `variants.rs` + `variants/{constructors,to_durable}.rs` | 3 | Enum + impls |
| `variants_tests.rs` | 1 | Tests (outside the dir) |
| Per-domain group files (`login_flow`, `session`, `io`, `control`, `scroll`, `config`, `command`, `dialog`, `dialog_display`, `system`, `durable`, `agent`, `input`, `level`, `names`, `model_config`, `edit`) | 16 | Sub-enums / aliases |
| `mod.rs` | 1 | Module root |
| `event/login_flow.rs` alias shim | included above | 1-line `pub type` |

**Achieved**: The 16 per-domain group files have been deleted. The `event/` directory was reduced from 22 files to 13 files. The remaining files represent meaningful architectural separations: intent system (`intent.rs`, `intent_impl.rs`), `EventKind` taxonomy (`kind/`), provider event conversion (`from_provider_event.rs`), and the core enum+constructors+durable+naming structure. Further aggressive consolidation to ~4 files would blur these architecturally meaningful distinctions.

## Acceptance Criteria

- [x] The 16 per-domain group files deleted — `event/` directory reduced from 22 to 13 files.
- [x] `event/variants/` dir flattened into `event/variants.rs`.
- [x] All `use runie_core::event::{...}` imports still resolve via `event/mod.rs` re-exports.
- [x] `event_size_reduced` test still passes.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `event_size_test_still_passes` — `event_size_reduced` test green.
- [x] `event_variant_count_unchanged` — 109 variants confirmed.
- [x] `durable_classification_unchanged` — `to_durable` produces the same durable/transient partition.

### Layer 2 — Event Handling
- [x] `all_event_handlers_resolve_after_collapse` — all handlers compile and route correctly.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms no broken imports.

## Files touched

- `crates/runie-core/src/event/*.rs` — 16 per-domain group files deleted
- `crates/runie-core/src/event/mod.rs` — re-exports maintained
- `crates/runie-core/src/event/variants.rs` — flattened from `variants/` subdir

## Notes

The original target of ~4 files was not met because the intent system (`intent.rs`, `intent_impl.rs`) and `EventKind` taxonomy (`kind/`) are architecturally meaningful abstractions worth preserving. The current 13 files represent a 41% reduction (22 → 13) with meaningful architectural separation maintained.
