# Collapse `Event`/`Intent`/`EventKind` taxonomies

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Depends on**: consolidate-actor-runtime-on-ractor
**Blocks**: none

## Description

`crates/runie-core/src/event/variants.rs` defines an `Event` enum with approximately 431 variants across 495 lines. `crates/runie-core/src/event/intent.rs` defines an `Intent` enum with approximately 286 variants across 457 lines, serving as a near 1:1 mirror of `Event`. `crates/runie-core/src/event/intent_impl.rs` (438 lines) manually maps every `Event` variant to its corresponding `Intent`. On top of that, `crates/runie-core/src/event/kind/mod.rs` (404 lines) classifies events with `matches!` predicates, and `crates/runie-core/src/update/dispatch.rs` adds yet another `EventCategory` taxonomy. This redundancy is error-prone and expensive to maintain. This task collapses the taxonomies by choosing a single canonical representation: either (a) nesting domain sub-enums inside `Event` and deriving `Intent`/`EventKind` from one source, or (b) making `Intent` the canonical request type and wrapping it in `Event`. The manual mirror is removed, all match sites are updated, and the workspace remains compiling.

## Acceptance Criteria

- [ ] A single canonical taxonomy is chosen and documented in the task notes.
- [ ] The manual `Event` → `Intent` mapping in `intent_impl.rs` is removed.
- [ ] `EventKind` and `EventCategory` classifications are derived from or folded into the canonical taxonomy.
- [ ] All `match` and `matches!` sites across `crates/runie-core` and downstream crates are updated to use the new structure.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_to_intent_roundtrip` — Verifies that every `Event` variant round-trips to the expected `Intent` (or canonical equivalent) and back without manual mapping tables.

### Layer 2 — Event Handling
- [ ] `dispatch_routes_nested_event` — Verifies that the update dispatcher routes a nested domain event to the correct handler after the taxonomy collapse.

### Layer 3 — Rendering
- [ ] N/A — Rendering code consumes the canonical event type unchanged; no widget-level changes are required.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `all_events_have_intent_or_kind` — Smoke test that enumerates the generated event taxonomy and asserts every variant has a deterministic classification and no variant is orphaned.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/intent.rs`
- `crates/runie-core/src/event/intent_impl.rs`
- `crates/runie-core/src/event/kind/mod.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/lib.rs` (if module structure changes)
- `crates/runie-core/src/**/*.rs` (all match sites)
- `crates/runie-tui/src/**/*.rs` (if it matches on `Event`/`Intent`)
- `crates/runie-provider/src/**/*.rs` (if it matches on `Event`/`Intent`)

## Notes

Option (a) is preferred if most match sites already operate on `Event`, because nested sub-enums preserve exhaustiveness checking while letting macros derive `Intent` and `EventKind`. Option (b) is preferred if most callers already produce `Intent` values and only the dispatcher wraps them. Rejected alternative: keeping all three enums and adding a fourth generated taxonomy — that would increase maintenance burden. Out of scope: changing the actor runtime itself (handled by `consolidate-actor-runtime-on-ractor`) or adding new event variants.
