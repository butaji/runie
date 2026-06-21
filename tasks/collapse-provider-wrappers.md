# Collapse provider abstraction wrappers

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Provider construction has multiple wrapper layers: `Provider` trait → `ProviderFactory` trait → `DynProvider` concrete wrapper → `DynProviderFactory` → `BuiltProvider` → retry wrapper. `runie-provider` both defines `DynProvider` and re-exports `ProviderError` so `runie-agent` can avoid a deep dependency. A headless binary traverses `HeadlessRuntime` → `ProviderActor` → `ProviderFactory` → `DynProvider` → `OpenAiProvider` wrapped in `RetryProvider`.

## Acceptance Criteria

- [ ] `runie-core` keeps the abstract `Provider` trait and metadata registry.
- [ ] Concrete provider construction lives in `runie-provider` only.
- [ ] `BuiltProvider` and `DynProvider` collapse into one concrete handle type.
- [ ] `ProviderActor` and headless runtime use the same construction path.
- [ ] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_handle_builds_openai_provider` — the collapsed handle still constructs an OpenAI-compatible provider from config.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_with_collapsed_provider` — run a headless turn through the new handle and assert output matches fixture expectations.
- [ ] `provider_actor_reuses_handle` — spawn `ProviderActor` and confirm it uses the same construction path as headless.

## Files touched

- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/actors/provider/factory.rs`
- `crates/runie-core/src/provider_registry/mod.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-provider/src/factory.rs`
- `crates/runie-agent/src/headless.rs`
- `crates/runie-agent/src/headless_runtime.rs`

## Notes

Coordinate with `unify-provider-modules`. The crate boundary should be: `runie-core` = trait + registry; `runie-provider` = implementations + builder.
