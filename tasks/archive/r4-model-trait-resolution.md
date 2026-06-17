# Model Trait Resolution

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: model-capability-flags, r4-orchestrator-domain-types
**Blocks**: r4-orchestrator-actor, r4-one-shot-orchestrator-llm

## Description

Build a resolver that maps an abstract **model trait** (e.g. `Reasoning`,
`Code`, `Fast`, `Vision`, `LongContext`) to a concrete configured model. The
Orchestrator uses this to pick the best model for each subtask and synthesis.

## What was implemented

- `crates/runie-core/src/trait_resolver.rs` (new)
- `ModelProfile` — maps `(provider, model)` to a set of `ModelTrait`s; auto-derives
  traits from `ModelCapabilities` when no explicit traits are declared.
- `ModelResolver` — resolves trait requests to profiles using a **tier-based**
  scoring system:
  - Tier 0: exact single-trait match
  - Tier 1: partial match with ≥1 non-General extra trait
  - Tier 2: partial match with General + one other trait
  - Tie-break: priority list → input Vec order
- `ResolverError` — `NoMatch` and `NoModelsConfigured` variants with `Display`.
- `ModelResolver::from_catalog(info: &[ModelInfo])` — builds profiles from the
  model catalog with auto-derived traits.

## Acceptance Criteria

- [x] `ModelTrait` enum exists with `General`, `Reasoning`, `Code`, `Fast`,
  `Vision`, `LongContext` (already in `orchestrator.rs`).
- [x] Each `ModelProfile` declares a `HashSet<ModelTrait>` (stored as `Vec`).
- [x] `ModelResolver` takes a `Vec<ModelProfile>` and a trait request and returns
  the best match.
- [x] Matching is deterministic: exact single-trait match wins; otherwise the
  profile with more non-General extra traits wins; ties broken by priority list.
- [x] If no model matches, return `ResolverError::NoMatch` with helpful message.
- [x] Provider preference flags are ignored at this layer.
- [x] `cargo test --workspace` passes.

## Tests (Layer 1 — State/Logic)

- `exact_trait_wins` — o3-mini wins for `Reasoning` over gpt-4o
- `general_trait_falls_back_to_any` — gpt-4o wins for `General`
- `general_falls_back_to_any_with_multiple` — single-trait `[General]` beats
  `[General, Vision]` (exact General match wins)
- `most_matching_traits_wins_on_partial_match` — `[Reasoning]` beats `[General]`
  for `Reasoning` requests (tier 1 beats tier 2)
- `priority_breaks_ties` — priority list respects order
- `no_match_returns_error` — `NoMatch` error for unsupported trait
- `empty_resolver_returns_no_models` — `NoModelsConfigured` error
- `profile_key` / `profile_has_trait` — helper correctness
- `model_profile_from_info` — auto-derivation from `ModelInfo`
- `reasoning_model_derives_correct_traits` — reasoning models get `Reasoning`,
  `LongContext` (200k), `General`; not `Fast` or `Vision`
- `resolve_all_deduplicates` — resolves multiple traits to profiles
- `resolver_error_display` — error `Display` format

## Files touched

- `crates/runie-core/src/trait_resolver.rs` — new module
- `crates/runie-core/src/lib.rs` — `pub mod trait_resolver`

## Out of scope

- Cost/routing optimization.
- Streaming provider selection logic.
