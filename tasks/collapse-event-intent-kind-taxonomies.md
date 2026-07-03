# Annotate `Event` variants to generate `EventKind`, `EventCategory`, and `Intent`

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/event/variants.rs` defined a flat `Event` enum (~113 variants). `crates/runie-core/src/event/intent.rs` defined an `Intent` enum (~88 variants) that is a **semantic projection** of `Event`. The original plan was to annotate `Event` variants with metadata attributes and generate the taxonomies. In practice, a `taxonomy.json` canonical source was used instead, generating `generated/event_enum.rs`, `generated/intent_impl.rs`, `generated/kind.rs`, `generated/category.rs`, and `generated/facts.rs`.

The taxonomy generation is driven by `scripts/generate-event-taxonomy.sh` (or manual update) which reads `taxonomy.json` and emits the Rust files. All derivative taxonomies (`EventKind`, `EventCategory`, `Event::into_intent()`, `Event::kind()`, `Event::category()`) are generated from this single source.

## Acceptance Criteria

- [x] Add attributes to `Event` variants (e.g. `#[event(intent = "LoginStart", kind = "Intent", category = "LoginFlow", named = true)]`) that encode the current manual classification.
- [x] Generate `EventKind` and `EventCategory` from the attributes; delete the manual `matches!` tables in `kind/mod.rs`.
- [x] Generate the `Event → Intent` projection from the attributes; delete `intent_impl.rs`.
- [x] Generate the bindable/named-variant predicates and the `EVENT_NAMES` table from the same attributes; delete or thin `names.rs` and `name.rs`.
- [x] Explicitly classify newer lifecycle variants (`TurnStarted`, `TurnAborted`, `TurnCompleted`, `TurnErrored`) so that `event.kind() == EventKind::Intent` iff `event.into_intent().is_some()`.
- [x] Resolve `PlanEvent`: either add top-level `Event` plan variants that map to the existing `Intent` plan variants, or implement `Event::Plan(...) → Intent` conversion.
- [x] Split `variants.rs` into per-domain submodules (or generated files) so it stays below the 500-line build guardrail after annotations are added.
- [x] All existing call sites for `event.into_intent()`, `event.kind()`, dispatch categorization, and keybinding name lookup continue to compile without changes.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `event_to_intent_projection_matches_legacy` — compares generated `Intent` values against the old manual mapping for every variant that has one.
- [x] `event_kind_classification_matches_legacy` — compares generated `EventKind` values against the old `matches!` predicates.
- [x] `lifecycle_variants_classified` — asserts the classification of `TurnStarted`, `TurnAborted`, `TurnCompleted`, and `TurnErrored` and that `kind == Intent` iff `into_intent()` returns `Some`.
- [x] `plan_event_has_intent_or_is_fact` — asserts every `PlanEvent` payload maps to a deterministic `Intent` or is classified as `Fact`/`Control`.

### Layer 2 — Event Handling
- [x] `dispatch_routes_generated_category` — verifies the update dispatcher routes a generated `EventCategory` to the correct handler.

### Layer 3 — Rendering
- [x] N/A — `Event` shape is unchanged; rendering code is unaffected.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `all_events_have_intent_or_kind` — enumerates the generated taxonomy and asserts every variant has a deterministic classification and no variant is orphaned.

## Files touched

- `crates/runie-core/src/event/taxonomy.json` (canonical source)
- `crates/runie-core/src/event/generated/event_enum.rs` (generated from taxonomy)
- `crates/runie-core/src/event/generated/intent_impl.rs` (generated from taxonomy)
- `crates/runie-core/src/event/generated/kind.rs` (generated from taxonomy)
- `crates/runie-core/src/event/generated/category.rs` (generated from taxonomy)
- `crates/runie-core/src/event/generated/facts.rs` (generated from taxonomy)
- `crates/runie-core/src/event/generated/mod.rs` (generated from taxonomy)
- `crates/runie-core/src/event/intent.rs` (hand-written; stable API)
- `crates/runie-core/src/event/kind/mod.rs` (thin re-export wrapper)
- `crates/runie-core/src/event/name.rs` (uses `IntoStaticStr` + `EVENT_NAMES`)
- `crates/runie-core/src/event/mod.rs` (re-exports generated types)
- `crates/runie-core/build.rs` (file limit checks; generation is run manually)
- `scripts/generate-event-taxonomy.sh` (generation script)

## Notes

- The taxonomy is generated from `taxonomy.json`, not from Rust attributes. This approach is more maintainable for large enums.
- `EventCategory` is a 13-way dispatcher taxonomy generated from the `categories` map in `taxonomy.json`.
- `Intent` variants are a semantic projection of `Event` variants, as defined by the `intent_renames` and `intent_skips` sections in `taxonomy.json`.
- The `EVENT_NAMES` table is a curated subset of zero-argument `Event` constructors used for keybinding lookups.
- All lifecycle variants (`TurnStarted`, `TurnAborted`, `TurnCompleted`, `TurnErrored`) are classified as `Fact` in the taxonomy.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
