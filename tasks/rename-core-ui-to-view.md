# Rename runie-core `ui/` to `view/`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: fold-state-into-model-state
**Blocks**: none

## Description

`crates/runie-core/src/ui/` (1041 LOC across `elements.rs`, `transform.rs`, `posts.rs`, `mod.rs`) is the **view-model**: pure domain projection (`AppState` → `Element`/`Feed`/`Post` tree, cached via `LazyCache`). It contains zero rendering code — no Ratatui imports. The actual UI (Ratatui widgets, `draw_snapshot`) lives in `crates/runie-tui/src/ui/` (497 LOC).

Two `ui/` directories in two crates with different meanings is confusing. The core one is not "UI" — it is the immutable view description that the TUI consumes. Renaming to `view/` (or `elements/`) makes the IO | Domain | UI split legible at the directory level: `view/` is the domain's pure projection, `runie-tui/src/ui/` is the rendering.

## Acceptance Criteria

- [ ] `crates/runie-core/src/ui/` renamed to `crates/runie-core/src/view/`.
- [ ] `lib.rs` declares `pub mod view;` and re-exports `Element`, `Feed`, `LazyCache`, `Post`, `PostBuilder`, `PostKind` from `view::`.
- [ ] All `use crate::ui::` and `use runie_core::ui::` imports rewritten to `view::`.
- [ ] `crates/runie-tui/src/core_ui/` re-export shim updated to `pub use runie_core::view::{...}` (or inlined per `inline-or-document-core-ui-shim`).
- [ ] `docs/Architecture.md` updated: "UI layer" → clarifies `view/` (domain projection) vs `runie-tui/src/ui/` (rendering).
- [ ] `rg "crate::ui::|runie_core::ui::" crates/` returns zero hits (all migrated to `view::`).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `view_transform_produces_elements` — `transform::to_elements` still produces the same `Element` vec after rename.
- [ ] `lazy_cache_invalidates_on_gen_change` — `LazyCache` behavior unchanged.

### Layer 2 — Event Handling
- N/A — pure rename, no event logic.

### Layer 3 — Rendering
- [ ] `draw_snapshot_consumes_view_elements` — `runie-tui/src/ui::draw_snapshot` still renders from the renamed `view::Element` type.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-core/src/ui/` → rename to `crates/runie-core/src/view/` (4 files)
- `crates/runie-core/src/lib.rs` — `pub mod view;` + re-exports
- All files importing `crate::ui::` or `runie_core::ui::` (grep-driven)
- `crates/runie-tui/src/core_ui/mod.rs` — update re-export source
- `docs/Architecture.md` — clarify `view/` vs `ui/` naming

## Notes

Use `git mv` to preserve history. Depends on `fold-state-into-model-state` so the rename does not collide with the state-module move in the same commit window. This is distinct from `inline-or-document-core-ui-shim` (which targets the `runie-tui/src/core_ui/` re-export alias, not the core `ui/` dir name). The `ui/dsl_test.rs` test file moves with the dir and is renamed `view/dsl_test.rs`. Rejected alternative: `elements/` — rejected because `transform.rs` and `posts.rs` are not "elements", they are the view projection; `view/` captures the MVU role.
