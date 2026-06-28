# Annotate `Event` variants to generate `EventKind`, `EventCategory`, and `Intent`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: collapse-actor-handles-to-typed-map
**Blocks**: none

## Description

`crates/runie-core/src/event/variants.rs` defines a flat `Event` enum (~113 variants). `crates/runie-core/src/event/intent.rs` defines an `Intent` enum (~88 variants) that is a **semantic projection** of `Event`, not a mechanical mirror: many variants are renamed (`Event::Start` → `Intent::LoginStart`, `Event::SwitchTheme` → `Intent::SetTheme`) and Facts have no `Intent` counterpart. `crates/runie-core/src/event/intent_impl.rs` (438 lines) manually maps `Event` to `Intent`, `crates/runie-core/src/event/kind/mod.rs` manually classifies `EventKind`, and `crates/runie-core/src/update/dispatch.rs` maintains a parallel 13-way `EventCategory` taxonomy. Additionally, `event/names.rs` and `event/name.rs` are manual mirrors used for keybinding/command names.

The goal is to keep the flat `Event` enum as the canonical source of truth, annotate its variants with metadata that describes their `Intent` projection, `EventKind`, `EventCategory`, and whether they are bindable/named, and generate the derivative taxonomies from those annotations.

## Acceptance Criteria

- [ ] Add attributes to `Event` variants (e.g. `#[event(intent = "LoginStart", kind = "Intent", category = "LoginFlow", named = true)]`) that encode the current manual classification.
- [ ] Generate `EventKind` and `EventCategory` from the attributes; delete the manual `matches!` tables in `kind/mod.rs`.
- [ ] Generate the `Event → Intent` projection from the attributes; delete `intent_impl.rs`.
- [ ] Generate the bindable/named-variant predicates and the `EVENT_NAMES` table from the same attributes; delete or thin `names.rs` and `name.rs`.
- [ ] Explicitly classify newer lifecycle variants (`TurnStarted`, `TurnAborted`, `TurnCompleted`, `TurnErrored`) so that `event.kind() == EventKind::Intent` iff `event.into_intent().is_some()`.
- [ ] Resolve `PlanEvent`: either add top-level `Event` plan variants that map to the existing `Intent` plan variants, or implement `Event::Plan(...) → Intent` conversion.
- [ ] Split `variants.rs` into per-domain submodules (or generated files) so it stays below the 500-line build guardrail after annotations are added.
- [ ] All existing call sites for `event.into_intent()`, `event.kind()`, dispatch categorization, and keybinding name lookup continue to compile without changes.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `event_to_intent_projection_matches_legacy` — compares generated `Intent` values against the old manual mapping for every variant that has one.
- [ ] `event_kind_classification_matches_legacy` — compares generated `EventKind` values against the old `matches!` predicates.
- [ ] `lifecycle_variants_classified` — asserts the classification of `TurnStarted`, `TurnAborted`, `TurnCompleted`, and `TurnErrored` and that `kind == Intent` iff `into_intent()` returns `Some`.
- [ ] `plan_event_has_intent_or_is_fact` — asserts every `PlanEvent` payload maps to a deterministic `Intent` or is classified as `Fact`/`Control`.

### Layer 2 — Event Handling
- [ ] `dispatch_routes_generated_category` — verifies the update dispatcher routes a generated `EventCategory` to the correct handler.

### Layer 3 — Rendering
- [ ] N/A — `Event` shape is unchanged; rendering code is unaffected.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `all_events_have_intent_or_kind` — enumerates the generated taxonomy and asserts every variant has a deterministic classification and no variant is orphaned.

## Files touched

- `crates/runie-core/src/event/variants.rs` (annotate; split if needed)
- `crates/runie-core/src/event/intent.rs` (becomes generated or re-exported)
- `crates/runie-core/src/event/intent_impl.rs` (delete)
- `crates/runie-core/src/event/kind/mod.rs` (becomes a thin wrapper)
- `crates/runie-core/src/event/names.rs` (delete or generate)
- `crates/runie-core/src/event/name.rs` (delete or generate)
- `crates/runie-core/src/update/dispatch.rs` (consume generated classification)
- `crates/runie-macros/src/event.rs` or a new generator module
- `crates/runie-core/build.rs` (if generation happens at build time)

## Notes

- This task intentionally does **not** restructure the flat `Event` enum. Nesting domain sub-enums is a useful future cleanup but it changes every `match` site; keep it separate.
- The derive macro/generator must respect the build guardrails (file/function length, complexity). If a generated file would exceed limits, split the generated output by domain.
- `EventCategory` is currently a 13-way dispatcher taxonomy. Decide whether to collapse it to a smaller set derived from `EventKind` + domain tags, or keep it as a generated separate enum.
- Rejected alternative: deleting `Intent` and routing on `Event` directly. Many DSL sites already speak in terms of `Intent`, so removing it would force a larger rewrite.
- Out of scope: changing the actor runtime or adding new event variants.
