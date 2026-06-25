# Consolidate dual-path foo.rs + foo/ module pairs

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Seven modules in `crates/runie-core/src/` used the dual-path layout (`foo.rs` declaring `mod foo/` submodules) instead of the conventional `foo/mod.rs` style. All have been migrated to the `foo/mod.rs` style.

| Module | Status |
|--------|--------|
| `tool_markers.rs` → `tool_markers/mod.rs` | Done |
| `tool_parser.rs` → `tool_parser/mod.rs` | Done |
| `provider_registry.rs` → `provider_registry/mod.rs` | Done |
| `model_catalog.rs` → `model_catalog/mod.rs` | Done |
| `login_config.rs` → `login_config/mod.rs` | Done |
| `message.rs` → `message/mod.rs` | Done |
| `model/cache.rs` → `model/cache/mod.rs` | Done |

Note: `config.rs` + `config/` remains as a dual-path pair (not listed in the original 7). This was not part of the original task scope.

## Acceptance Criteria

- [x] For each of the 7 pairs, `foo.rs` becomes `foo/mod.rs` (its content is unchanged except for relative path fixes if any).
- [x] No `foo.rs` + `foo/` dual-path pair remains among the 7 listed modules.
- [x] All `use crate::foo::...` and `use runie_core::foo::...` imports still resolve (no public path changes).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 4 — Smoke / Crash
- [x] `smoke_module_paths_resolve_after_consolidation` — `cargo check --workspace` green.

## Files touched

All 7 module pairs were migrated to `mod.rs` style:
- `tool_markers/mod.rs`
- `tool_parser/mod.rs`
- `provider_registry/mod.rs`
- `model_catalog/mod.rs`
- `login_config/mod.rs`
- `message/mod.rs`
- `model/cache/mod.rs`

## Notes

Remaining dual-path pair: `config.rs` + `config/` (not part of this task scope).
