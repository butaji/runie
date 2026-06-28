# Deduplicate provider registry data

**Status**: done
**Milestone**: R4
**Category**: Provider
**Priority**: P2

**Depends on**: prune-dead-provider-code-and-rig-core-dependency, unify-provider-modules
**Blocks**: none

## Description

Eliminate the near-identical copies of provider registry data in `runie-core` and `runie-provider` by keeping one canonical implementation in `runie-provider` and re-exporting it from `runie-core`. The two files differ only in `include_str!(...)` relative paths and minor test comments, so consolidation is mechanical once the dead provider code and module split are resolved.

## Acceptance Criteria

- [x] Choose `crates/runie-provider/src/registry/registry_data.rs` as the canonical source of truth.
- [x] Re-export the registry data module from `runie-core` instead of maintaining a second copy.
- [x] Delete `crates/runie-core/src/provider/registry_data.rs`.
- [x] Update any `include_str!` paths in the canonical file so embedded YAML models load correctly from both crates.
- [x] Preserve all existing model structs and the embedded `resources/models/*.yaml` list.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `registry_data_loads_same_models_from_both_crates` — loads the embedded model list through the `runie-core` re-export and the `runie-provider` original and asserts identical IDs, display names, and capabilities.

### Layer 2 — Event Handling
- [x] N/A — registry data is pure configuration; no input events are handled.

### Layer 3 — Rendering
- [x] N/A — no TUI rendering is involved.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `provider_registry_roundtrip` — starts the runtime, resolves a provider by registry key, and confirms the model metadata returned matches the canonical registry data.

## Files touched

- `crates/runie-provider/src/registry/registry_data.rs`
- `crates/runie-core/src/provider/registry_data.rs` (delete)
- `crates/runie-core/src/provider/mod.rs`
- `crates/runie-core/src/lib.rs` (if re-export path changes)
- `Cargo.toml` workspace paths (if `include_str!` requires adjusted source roots)

## Notes

The duplication was introduced while `runie-provider` was being split out of `runie-core`. Because both files share the same YAML model structs, a re-export avoids divergence. Rejected alternative: keeping both files in sync with a build-time copy — it is fragile and adds unnecessary build complexity. Out of scope: changing the YAML schema or adding new providers; this task only moves and re-exports existing data.
