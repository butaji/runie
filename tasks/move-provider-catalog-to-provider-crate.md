# Move provider registry and model catalog into runie-provider

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: unify-provider-modules
**Blocks**: none

## Description

The domain crate `runie-core` carries ~840 LOC of provider/model knowledge that belongs in `runie-provider`:

| File | LOC | Content |
|------|-----|---------|
| `provider_registry.rs` | 435 | `ProviderMeta`, `find_provider`, known-provider metadata table |
| `model_catalog.rs` | 405 | Model catalog, trait resolution, capability flags |

`runie-provider` already exists and owns the `Provider` trait, concrete clients, and model definitions. The registry and catalog are provider-crate concerns (they describe provider capabilities and model traits), not domain concerns. Keeping them in core means the domain crate depends on provider-specific metadata it doesn't use for state transitions — a layering smell. Move both into `runie-provider`; `runie-core` keeps only the `Provider` trait + `DynProvider` + `ResponseChunk` (the abstract interface the domain needs to talk to).

This is deeper than `unify-provider-modules` (which consolidates core's own 4 provider files into a `provider/` dir). Do that consolidation first, then move the result into `runie-provider`.

## Acceptance Criteria

- [ ] `crates/runie-core/src/provider_registry.rs` deleted; contents moved to `crates/runie-provider/src/registry.rs`.
- [ ] `crates/runie-core/src/model_catalog.rs` deleted; contents moved to `crates/runie-provider/src/catalog.rs`.
- [ ] `runie-core` retains only: `Provider` trait, `DynProvider`, `ProviderError`, `ResponseChunk`, `Message` (the abstract interface).
- [ ] `runie-provider` re-exports `ProviderMeta`, `find_provider`, `ModelCatalog`, capability flags, trait resolution.
- [ ] `runie-core` and `runie-agent` depend on `runie-provider` for catalog/registry access (or receive them via dependency injection if circular-dep concerns arise).
- [ ] No circular dependency introduced: `runie-provider` does not depend on `runie-core` for these types (it may already depend on core for the `Provider` trait — that direction is fine).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `find_provider_returns_meta_from_provider_crate` — `runie_provider::find_provider("openai")` returns correct `ProviderMeta` after move.
- [ ] `model_catalog_resolves_traits` — trait resolution (e.g. "reasoning" → concrete model) works from `runie_provider::catalog`.

### Layer 2 — Event Handling
- [ ] N/A — pure module move, no event logic.

### Layer 3 — Rendering
- [ ] N/A — catalog/registry are data; rendering tests in runie-tui cover indirectly.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms no broken imports or circular deps.

## Files touched

- `crates/runie-core/src/provider_registry.rs` → delete (move to `runie-provider/src/registry.rs`)
- `crates/runie-core/src/model_catalog.rs` → delete (move to `runie-provider/src/catalog.rs`)
- `crates/runie-provider/src/lib.rs` — declare new modules + re-exports
- `crates/runie-provider/src/registry.rs` → new
- `crates/runie-provider/src/catalog.rs` → new
- `crates/runie-core/src/lib.rs` — remove `pub mod provider_registry;`, `pub mod model_catalog;`, update re-exports
- `crates/runie-core/Cargo.toml` — add `runie-provider` dep if not present (for trait only)
- All files importing `runie_core::provider_registry::` / `runie_core::model_catalog::` (grep-driven)
- `crates/runie-core/tests/arch_guardrails.rs` — update path strings

## Notes

Depends on `unify-provider-modules` so the move starts from the consolidated `provider/` dir, not 4 scattered root files. If `runie-agent` needs the catalog for trait resolution during team-mode orchestration, it should depend on `runie-provider` directly (it likely already does for the `Provider` trait). The ~840 LOC reduction in `runie-core` is the largest single domain-shrink available. Rejected alternative: keep catalog in core "for trait resolution" — rejected because trait resolution is a provider-capability question, not a domain-state question.
