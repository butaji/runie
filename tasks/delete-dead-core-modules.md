# Delete 5 dead modules in runie-core

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Five `pub mod X;` modules in `crates/runie-core/src/lib.rs` have zero external callers anywhere in the workspace (verified by `rg` for both the module path and every exported symbol — only self-references and own `#[cfg(test)]` blocks match). They are scaffolding-for-later or stale duplicates, not live code. Together they account for ~1,148 LOC of dead surface that pollutes `rg`/glob results and misleads readers.

| Module | LOC | Why dead |
|--------|-----|----------|
| `hooks.rs` | 349 | `HookEvent` / `HookRegistry` / `HookDecision` never imported. The active plan `tasks/archive/adopt-hooks-registry.md` is still `todo`, so this is unused scaffolding. |
| `skill/mod.rs` (singular) | 318 | `SkillRegistry` / `SkillSummary` only used in own `#[cfg(test)]`. The live skill system is the plural `skills/` module (`load_all`, `Skill`, `build_skills_context`). The parallel singular name is confusing. |
| `retry.rs` | 233 | `RetryConfig` here is unused. The real retry logic lives in `runie-provider/src/retry.rs` (`RetryProvider`, own `RetryConfig`). Duplicate type, no callers. |
| `location.rs` | 195 | `parse_location` only appears in its own doc-comment example. No `crate::location::` / `runie_core::location::` references. |
| `utils.rs` | 53 | `truncate` / `join_optional` never imported (the `.truncate()` hits elsewhere are `Vec`/`String` methods, not this module). |

`mcp.rs` (482 LOC, mostly stub) is **excluded** — it is already tracked by `gate-or-implement-mcp-client.md` which requires a wire-or-delete decision, not a pure deletion.

## Acceptance Criteria

- [x] `crates/runie-core/src/hooks.rs` deleted; `pub mod hooks;` removed from `lib.rs`.
- [x] `crates/runie-core/src/skill/` directory deleted; `pub mod skill;` removed from `lib.rs`.
- [x] `crates/runie-core/src/retry.rs` deleted; `pub mod retry;` removed from `lib.rs`.
- [x] `crates/runie-core/src/location.rs` deleted; `pub mod location;` removed from `lib.rs`.
- [x] `crates/runie-core/src/utils.rs` deleted; `pub mod utils;` removed from `lib.rs`.
- [x] `rg "crate::hooks|runie_core::hooks|HookEvent|HookRegistry" crates/` returns zero hits.
- [x] `rg "crate::skill::|runie_core::skill::|SkillSummary" crates/` returns zero hits (the plural `crate::skills::` must still resolve).
- [x] `rg "crate::retry|runie_core::retry" crates/runie-core/` returns zero hits (the `runie-provider/src/retry.rs` stays).
- [x] `rg "crate::location|runie_core::location|parse_location" crates/` returns zero hits.
- [x] `rg "crate::utils::|runie_core::utils::|utils::truncate|utils::join_optional" crates/` returns zero hits.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure deletion of unreferenced modules; no behavior to assert.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_workspace_builds_after_dead_module_purge` — `cargo check --workspace` green after the 5 deletions.
- [x] `smoke_skills_module_still_loads` — `crate::skills::load_all` still compiles and returns the expected skill list (the plural module is untouched).

## Files touched

- `crates/runie-core/src/hooks.rs` (delete)
- `crates/runie-core/src/skill/` (delete dir)
- `crates/runie-core/src/retry.rs` (delete)
- `crates/runie-core/src/location.rs` (delete)
- `crates/runie-core/src/utils.rs` (delete)
- `crates/runie-core/src/lib.rs` (remove 5 `pub mod` lines)

## Notes

Verified via `rg "crate::X|runie_core::X::|SymbolName" crates/ --type rust` for each module and its exported symbols. The only matches were inside the module itself or in `lib.rs`. `mcp.rs` is deliberately excluded (see `gate-or-implement-mcp-client.md`). If the hooks registry is later adopted per `archive/adopt-hooks-registry.md`, it should be reintroduced under `harness_skills/` or a new module — not as a parallel `hooks.rs` duplicate of the live skill system.
