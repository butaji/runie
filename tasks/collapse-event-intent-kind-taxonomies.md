# Derive `Intent` and `EventKind` from the canonical `Event` enum

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/event/variants.rs` defines a flat `Event` enum with approximately 431 variants. `crates/runie-core/src/event/intent.rs` defines an `Intent` enum with approximately 286 variants that mirrors `Event`, and `crates/runie-core/src/event/intent_impl.rs` (438 lines) manually maps every `Event` variant to its `Intent` twin. `crates/runie-core/src/event/kind/mod.rs` (404 lines) repeats the same variant lists in `matches!` predicates, and `crates/runie-core/src/update/dispatch.rs` adds another `EventCategory` taxonomy.

Rather than restructuring `Event` in one risky pass, this task keeps the flat `Event` enum as the canonical source of truth and introduces derive macros (or a build-time generator) that produce `Intent` and `EventKind`/`EventCategory` from it. The manual mirrors in `intent_impl.rs` and `kind/mod.rs` are deleted, but existing `match` sites are left untouched until a follow-up task decides whether to nest domain sub-enums.

Current state as of this review:

- `event/mod.rs` still declares `pub(crate) mod intent_impl;`.
- `variants.rs` has no generation attributes.
- `kind/mod.rs` does not classify newer lifecycle variants such as `TurnStarted`, `TurnAborted`, `TurnCompleted`, and `TurnErrored`; they fall through to the default "intent" branch. The generator must either preserve this accidental behavior or deliberately fix it with tests.

## Acceptance Criteria

- [ ] A derive macro or build script generates `Intent` and `EventKind` from `Event` variants and their attributes.
- [ ] `crates/runie-core/src/event/intent_impl.rs` is deleted; no manual `Event → Intent` mapping remains.
- [ ] `crates/runie-core/src/event/kind/mod.rs` is reduced to re-exports or thin wrappers around the generated classification.
- [ ] All existing call sites for `event.into_intent()`, `event.kind()`, and dispatch categorization continue to compile without changes.
- [ ] Generated output is split by domain so that no generated file exceeds the 500-line build guardrail.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_to_intent_roundtrip` — verifies that every `Event` variant round-trips to the generated `Intent` and back without manual mapping tables.
- [ ] `event_kind_classification_matches_legacy` — compares generated `EventKind` values against the old `matches!` predicates for a representative set of variants.
- [ ] `lifecycle_variants_classified` — explicitly asserts the classification of `TurnStarted`, `TurnAborted`, `TurnCompleted`, and `TurnErrored` rather than relying on the accidental default.

### Layer 2 — Event Handling
- [ ] `dispatch_routes_generated_intent` — verifies the update dispatcher routes a generated `Intent` to the correct handler.

### Layer 3 — Rendering
- [ ] N/A — `Event` shape is unchanged; rendering code is unaffected.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `all_events_have_intent_or_kind` — enumerates the generated taxonomy and asserts every variant has a deterministic classification and no variant is orphaned.

## Files touched

- `crates/runie-core/src/event/variants.rs` (add attributes to drive generation)
- `crates/runie-core/src/event/intent.rs` (becomes a generated or re-exported type)
- `crates/runie-core/src/event/intent_impl.rs` (delete)
- `crates/runie-core/src/event/kind/mod.rs` (becomes a thin wrapper)
- `crates/runie-core/src/update/dispatch.rs` (consume generated classification)
- `crates/runie-macros/src/event.rs` or a new generator module
- `crates/runie-core/build.rs` (if generation happens at build time)

## Notes

- This task intentionally does **not** restructure the flat `Event` enum. Nesting domain sub-enums is a useful future cleanup but it changes every `match` site; keep it separate.
- The derive macro should respect the build guardrails (file/function length, complexity). If a generated file would exceed limits, split the generated output by domain.
- Rejected alternative: deleting `Intent` and routing on `Event` directly. Many DSL sites already speak in terms of `Intent`, so removing it would force a larger rewrite.
- Out of scope: changing the actor runtime (handled by `migrate-production-actors-to-ractor` and its children) or adding new event variants.
