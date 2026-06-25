# Inline or document runie-tui core_ui re-export shim

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/core_ui/mod.rs` is an 8-line re-export shim: `pub use runie_core::ui::{Element, Feed, LazyCache, Post, PostBuilder, PostKind};`. The doc comment claims moving the types into `runie-tui` is "blocked by orphan rules and the current crate dependency graph" but does not justify why callers inside `runie-tui` need a `crate::core_ui::` alias at all — they could `use runie_core::ui::Element;` directly and skip the indirection.

Current callers (8 internal sites in `runie-tui`): `ui/messages/mod.rs`, `ui/messages/nav.rs`, `ui/render_lines.rs`, `status_bar.rs`, `tests/render/vim_nav/wrap_mapping.rs`. No external crate imports `runie_tui::core_ui`.

Either the alias earns its keep (give a concrete reason) or it should be inlined.

## Acceptance Criteria

- [ ] **Option A (inline)**: 8 internal callers rewritten to `use runie_core::ui::...`; `core_ui/` deleted; `pub mod core_ui;` removed from `runie-tui/src/lib.rs`. OR
- [ ] **Option B (keep + document)**: doc comment updated with a concrete orphan-rule or dependency-graph blocker (not just a hand-wave), and the module stays.
- [ ] `rg "core_ui" crates/runie-tui/` either returns zero hits (option A) or every hit is justified by the documented blocker (option B).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — re-export alias, no logic.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [ ] `render_uses_canonical_element_type` — a render test that exercises `Element::AgentMessage` still passes after the import path switch (option A).

### Layer 4 — Smoke / Crash
- [ ] `smoke_tui_compiles_after_core_ui_inline` — `cargo check -p runie-tui` green.

## Files touched

- `crates/runie-tui/src/core_ui/mod.rs` (delete or document)
- `crates/runie-tui/src/lib.rs` (remove `pub mod core_ui;` if option A)
- 8 caller files in `crates/runie-tui/src/` (rewrite imports if option A)

## Notes

Same class as `delete-config-reload-shim`, `delete-path-utils-reexport`, and `delete-tui-ipc-reexport-shim`, but lower priority because the doc comment at least attempts a justification. Pick option A unless the orphan-rule blocker is real and demonstrated — the burden is on keeping the indirection, not on removing it.
