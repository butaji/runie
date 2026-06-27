# Move provider registry and model catalog into runie-provider

**Status**: blocked
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: unify-provider-modules, move-chatmessage-to-shared-crate
**Blocks**: none

## Blocked Reason

This task creates a circular dependency:

1. `runie-provider` implements `Provider` trait which uses `ChatMessage` from `runie-core`
2. `model_catalog` in `runie-core` calls `crate::provider::known_providers()`
3. Moving catalog to `runie-provider` would require `runie-core` to depend on `runie-provider`
4. `runie-provider` already depends on `runie-core` for `ChatMessage`

This creates: `runie-core` → `runie-provider` → `runie-core` (cycle)

## Alternative Approach

To complete this task, `ChatMessage` must move to a shared location:

**Option A**: Move `ChatMessage` to `runie-protocol`
- `runie-protocol` already exists for shared IPC types
- Add `ChatMessage`, `Part`, `Role`, `ToolCall` to `runie-protocol`
- `runie-provider` depends on `runie-protocol` (not `runie-core`) for message types
- `runie-core` depends on `runie-protocol` for message types
- `runie-core` can now depend on `runie-provider` for registry/catalog
- No circular dependency

**Option B**: Create new `runie-message` crate
- Similar to Option A but cleaner separation
- More work than using existing `runie-protocol`

## Description (when unblocked)

The domain crate `runie-core` carries ~840 LOC of provider/model knowledge that belongs in `runie-provider`:

| File | LOC | Content |
|------|-----|---------|
| `provider/registry.rs` | 327 | `ProviderMeta`, `find_provider`, known-provider metadata table |
| `provider/registry_data.rs` | 145 | YAML loading |
| `model_catalog/mod.rs` | 404 | Model catalog, trait resolution, capability flags |

`runie-provider` already exists and owns the `Provider` trait, concrete clients, and model definitions. The registry and catalog are provider-crate concerns (they describe provider capabilities and model traits), not domain concerns. Keeping them in core means the domain crate depends on provider-specific metadata it doesn't use for state transitions — a layering smell. Move both into `runie-provider`; `runie-core` re-exports from `runie-provider`.

## Acceptance Criteria (when unblocked)

- [ ] `ChatMessage` moved to `runie-protocol` (prerequisite)
- [ ] `crates/runie-core/src/provider/registry.rs` deleted; contents moved to `crates/runie-provider/src/registry/`
- [ ] `crates/runie-core/src/model_catalog/` deleted; contents moved to `crates/runie-provider/src/catalog/`
- [ ] `runie-core` re-exports from `runie-provider`: `ProviderMeta`, `find_provider`, `model_catalog`, etc.
- [ ] `runie-provider` re-exports from local modules
- [ ] No circular dependency: `runie-provider` → `runie-protocol` → `runie-core`, and `runie-core` → `runie-provider`
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo check --workspace` succeeds with no new warnings

## Files to touch (when unblocked)

- `crates/runie-protocol/src/` — add `ChatMessage`, `Part`, `Role`, `ToolCall` modules
- `crates/runie-core/src/message/` → delete (moved to protocol)
- `crates/runie-core/src/provider/` — remove registry files
- `crates/runie-core/src/model_catalog/` → delete (moved)
- `crates/runie-provider/src/registry/` → new
- `crates/runie-provider/src/catalog/` → new
- All files importing `ChatMessage` from `runie_core::message` → update to `runie_protocol::message`
- All files importing registry/catalog from `runie_core` → update to `runie_provider`
