# Unify 4 provider modules into `provider/` dir

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Four provider-related modules live at the `runie-core` src root with overlapping names: `provider.rs` (the `Provider` trait + `Message` enum), `provider_registry.rs` + `provider_registry/data.rs` (metadata for known providers), and `providers_dialog.rs` (the providers management dialog builder). Consolidate all four into a single `provider/` directory with `trait.rs`, `registry.rs`, and `dialog.rs` submodules. Supersedes the `provider_registry` item in `consolidate-dual-path-modules` — do this deeper merge instead of the shallow `foo.rs → foo/mod.rs` conversion for provider_registry.

## Acceptance Criteria

- [x] `provider.rs`, `provider_registry.rs`, `provider_registry/`, `providers_dialog.rs` removed from src root.
- [x] New `provider/` dir contains `mod.rs`, `provider_trait.rs` (renamed from trait.rs to avoid keyword), `registry.rs`, `registry_data.rs`, `dialog.rs`.
- [x] `lib.rs` exports `pub mod provider;` and re-exports `Provider`, `ProviderError`, `ResponseChunk`, `ProviderMeta`, `ModelMeta`, `find_provider`, `find_model`, `known_providers`, etc. from `provider::`.
- [x] All external call sites (`runie-agent`, `runie-provider`, `runie-tui`, `runie-server`) compile without path changes beyond the crate-root re-exports.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `provider_registry_find_provider_returns_meta` — find_provider("openai") still returns correct ProviderMeta after move. (covered by existing tests in registry.rs)
- [x] `provider_trait_implementors_compile` — existing MockProvider/real providers still implement the trait. (verified by cargo test)

### Layer 2 — Event Handling
- [x] N/A — pure file reorganization, no event logic changes.

### Layer 3 — Rendering
- [x] N/A — dialog builder is data-only; rendering tests in runie-tui cover it indirectly.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms no broken imports.

## Files touched

- `crates/runie-core/src/provider.rs` → delete (move to `provider/trait.rs`)
- `crates/runie-core/src/provider_registry.rs` → delete (move to `provider/registry.rs`)
- `crates/runie-core/src/provider_registry/data.rs` → delete (fold into `provider/registry.rs`)
- `crates/runie-core/src/providers_dialog.rs` → delete (move to `provider/dialog.rs`)
- `crates/runie-core/src/provider/mod.rs` → new
- `crates/runie-core/src/provider/trait.rs` → new
- `crates/runie-core/src/provider/registry.rs` → new
- `crates/runie-core/src/provider/dialog.rs` → new
- `crates/runie-core/src/lib.rs` — update module declarations + re-exports
- `crates/runie-core/tests/arch_guardrails.rs` — update path strings if needed

## Notes

This task supersedes the `provider_registry` portion of `consolidate-dual-path-modules`. If that task runs first, this becomes a second move; prefer running this instead of that item. The `data.rs` submodule (235 LOC) is small enough to inline directly into `registry.rs`. Rejected alternative: keep 4 separate root files — rejected because the naming collision (`provider` vs `provider_registry` vs `providers_dialog`) causes confusion and grep ambiguity.
