# Consolidate dual-path foo.rs + foo/ module pairs

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Seven modules in `crates/runie-core/src/` use the dual-path layout (`foo.rs` declaring `mod foo/` submodules) instead of the conventional `foo/mod.rs` style. The dual-path form is non-idiomatic and forces `rg`/glob/grep users to remember two locations per module. Pick one style workspace-wide and migrate.

| Current | LOC | `mod` declared in `foo.rs` |
|---------|-----|------------------------------|
| `tool_markers.rs` + `tool_markers/strip.rs` | 127 + 404 | `mod strip;` |
| `tool_parser.rs` + `tool_parser/minimax.rs` | 361 + 185 | `pub mod minimax;` |
| `provider_registry.rs` + `provider_registry/data.rs` | 435 + 235 | `mod data;` |
| `model_catalog.rs` + `model_catalog/configured.rs` | 405 + 85 | `pub mod configured;` |
| `login_config.rs` + `login_config/tests.rs` | 141 + 260 | `mod tests;` |
| `message.rs` + `message/parts.rs` | 342 + 56 | `pub mod parts;` |
| `model/cache.rs` + `model/cache/tests.rs` | ~470 + 50 | `mod tests;` |

Rust 2018+ supports both layouts; this is purely a consistency change. After migration, every module is a single `foo/mod.rs` + siblings. The `model/cache` pair was missed in the original audit and added by the ranked-review pass (finding F39).

## Acceptance Criteria

- [ ] For each of the 7 pairs, `foo.rs` becomes `foo/mod.rs` (its content is unchanged except for relative path fixes if any).
- [ ] No `foo.rs` + `foo/` dual-path pair remains in `crates/runie-core/src/`.
- [ ] All `use crate::foo::...` and `use runie_core::foo::...` imports still resolve (no public path changes).
- [ ] `arch_guardrails` test (if it lists module paths) still passes; update path strings if the guardrail list points at `foo.rs` rather than `foo/mod.rs`.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure file relocation; no logic change.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_module_paths_resolve_after_consolidation` — `cargo check --workspace` green; `rg "crate::tool_markers|crate::tool_parser|crate::provider_registry|crate::model_catalog|crate::login_config|crate::message|crate::model::cache" crates/` returns the same set of callers as before.

## Files touched

- `crates/runie-core/src/tool_markers.rs` → `tool_markers/mod.rs`
- `crates/runie-core/src/tool_parser.rs` → `tool_parser/mod.rs`
- `crates/runie-core/src/provider_registry.rs` → `provider_registry/mod.rs`
- `crates/runie-core/src/model_catalog.rs` → `model_catalog/mod.rs`
- `crates/runie-core/src/login_config.rs` → `login_config/mod.rs`
- `crates/runie-core/src/message.rs` → `message/mod.rs`
- `crates/runie-core/src/model/cache.rs` → `model/cache/mod.rs`
- `crates/runie-core/tests/arch_guardrails.rs` (if path strings need updating)

## Notes

Use `git mv` to preserve history. Do not combine with content changes — keep this commit purely mechanical so reviewers can verify "move only". After this lands, apply the same convention to the 6 `session_*.rs` files (see `group-session-modules-into-dir`).
