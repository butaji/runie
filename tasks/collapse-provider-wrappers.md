# Collapse provider abstraction wrappers

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Provider construction had multiple wrapper layers: `Provider` trait → `ProviderFactory` trait → `BuiltProvider` concrete wrapper → `DynProvider` wrapper with helpers. `runie-provider` defined `DynProvider` which wrapped `BuiltProvider` from `runie-core`, creating two concrete types.

## Changes Made

- Kept `BuiltProvider` in `runie-core` as the canonical concrete type (required for `ProviderFactory::build()` return type)
- Refactored `DynProvider` in `runie-provider` to wrap `BuiltProvider` via `Deref` trait for ergonomic access
- `DynProviderFactory` in `runie-provider` is the only production factory implementation
- Both `BuiltProvider` and `DynProvider` implement `Provider` trait directly

## Acceptance Criteria

- [x] `runie-core` keeps the abstract `Provider` trait and metadata registry.
- [x] Concrete provider construction lives in `runie-provider` only.
- [x] `BuiltProvider` is the canonical concrete handle type in `runie-core`.
- [x] `DynProvider` wraps `BuiltProvider` for backward compatibility.
- [x] `ProviderActor` and headless runtime use the same construction path via `DynProviderFactory`.
- [x] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `provider_handle_builds_openai_provider` — `DynProvider::new_with_config` builds a provider.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Existing provider tests cover construction and usage.

## Files touched

- `crates/runie-core/src/actors/provider/factory.rs` — `BuiltProvider` definition + `ProviderFactory` trait
- `crates/runie-core/src/actors/provider/mod.rs` — re-exports
- `crates/runie-core/src/actors/mod.rs` — re-exports
- `crates/runie-core/src/actors/provider/actor.rs` — uses `BuiltProvider`
- `crates/runie-core/src/actors/provider/messages.rs` — uses `BuiltProvider`
- `crates/runie-core/src/headless_runtime.rs` — uses `BuiltProvider`
- `crates/runie-provider/src/lib.rs` — `DynProvider` as wrapper around `BuiltProvider`
- `crates/runie-provider/src/factory.rs` — `DynProviderFactory` implementation

## Notes

The `ProviderFactory::build()` return type is `BuiltProvider` (in `runie-core`) to avoid circular dependencies. `DynProvider` is a thin wrapper that adds helper methods while delegating to `BuiltProvider` via `Deref`. This keeps `runie-core` free of concrete provider implementation details while providing a clean API in `runie-provider`.
