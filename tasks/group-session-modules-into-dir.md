# Group session_*.rs files into session/ directory

**Status**: todo
**Milestone**: R4
**Category**: Sessions
**Priority**: P1

**Depends on**: consolidate-dual-path-modules
**Blocks**: none

## Description

Six `session_*.rs` files live at the `crates/runie-core/src/` root (1,863 LOC), inflating the root file count to 60 and forcing readers to grep six separate prefixes. Grouping them under a single `session/` directory gives one import root (`crate::session::...`) and reduces root clutter by 6 entries.

| File | LOC |
|------|-----|
| `session.rs` (DTO) | 177 |
| `session_actor.rs` | 355 |
| `session_index.rs` | 341 |
| `session_replay.rs` | 343 |
| `session_store.rs` | 287 |
| `session_tree.rs` | 360 |

External callers exist: `runie-testing` imports `SessionStore`, `arch_guardrails.rs` references `session_replay.rs` by path, and `update/session.rs` references `session_tree::SessionTree`. The migration is mechanical (`git mv`, update `lib.rs` `pub mod` lines, update the few `crate::session_X` call sites to `crate::session::X`).

## Acceptance Criteria

- [ ] `crates/runie-core/src/session/{mod,actor,index,replay,store,tree}.rs` exist; the 6 root `session_*.rs` files are gone.
- [ ] `lib.rs` declares `pub mod session;` (replacing the 6 `pub mod session_*;` lines) and re-exports the same public types at `runie_core::session::*` so existing `use runie_core::SessionStore;` etc. still resolve — OR every call site is updated to the new path.
- [ ] `update/session.rs` references to `crate::session_tree::SessionTree` become `crate::session::tree::SessionTree` (or the re-export path).
- [ ] `runie-testing/src/fixtures.rs` (`use runie_core::session_store::SessionStore;`) still compiles.
- [ ] `arch_guardrails.rs` path string `"session_replay.rs"` becomes `"session/replay.rs"` (or `"session/replay/mod.rs"`).
- [ ] `rg "^crates/runie-core/src/session_[a-z]+\.rs$" crates/` returns zero hits.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `session_store_round_trips_after_relocation` — `SessionStore` still loads/saves a session through the new path (existing test, must stay green).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_session_imports_resolve` — `cargo check --workspace` green; the `runie-testing` crate still builds.

## Files touched

- `crates/runie-core/src/session.rs` → `session/mod.rs`
- `crates/runie-core/src/session_actor.rs` → `session/actor.rs`
- `crates/runie-core/src/session_index.rs` → `session/index.rs`
- `crates/runie-core/src/session_replay.rs` → `session/replay.rs`
- `crates/runie-core/src/session_store.rs` → `session/store.rs`
- `crates/runie-core/src/session_tree.rs` → `session/tree.rs`
- `crates/runie-core/src/lib.rs` (collapse 6 `pub mod` lines into `pub mod session;` + re-exports)
- `crates/runie-core/src/update/session.rs` (update `crate::session_tree::` → `crate::session::tree::`)
- `crates/runie-testing/src/fixtures.rs` (update import path)
- `crates/runie-core/tests/arch_guardrails.rs` (update path string)

## Notes

Use `git mv` to preserve history. Decide once whether to keep flat re-exports at `runie_core::SessionStore` (backward-compat) or update all call sites to the new path — the latter is cleaner but touches more files. Depends on `consolidate-dual-path-modules` so the two refactors land in a consistent order and don't conflict in `lib.rs`.
