# Unify duplicate module names across core and TUI

**Status**: partial
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Duplicate module names in `runie-core` and `runie-tui` create split-brain ownership. The 2026-06-28 review found the following pairs:

| Module | Core (`runie-core`) | TUI (`runie-tui`) | Status |
|---|---|---|---|
| `diff` | `diff.rs` (domain patch parse/apply) | `diff.rs` (gutter+span rendering) | Resolved in `tasks/archive/merge-diff-modules.md` |
| `ui` | `ui/` (Element/Feed/Transform view-model) | `ui/` (layout/messages/scroll render) | Resolved in `tasks/archive/rename-core-ui-to-view.md` |
| `ipc` | `ipc.rs` (TuiIpc) | `ipc.rs` (re-export shim) | Resolved in archived `inline-tui-ipc-reexport` / `fold-protocol-into-core` |
| `markdown` | `markdown/` (pure parsing) | `markdown_render.rs` (render adapter) | **Resolved** — TUI module was renamed |
| `theme`/`themes` | `themes.rs` (token list) | `theme/` (colors/glyph/loader/styles) | **Remaining** |

The remaining work is the `theme`/`themes` collision. `runie-core/src/themes.rs` defines the canonical `BUILTIN_THEMES` constant, which is used by core itself for validation (`crates/runie-core/src/settings/dialog.rs:290`, `crates/runie-core/src/update/dialog/open.rs:116`, `crates/runie-core/src/update/system.rs:121`) and re-exported by `runie-tui/src/theme/loader.rs:4`. Because core still owns the canonical list, moving it to TUI would create a circular dependency. The fix is to rename the core module so the names are unambiguous: core keeps the token list as `theme_tokens.rs`; TUI keeps `theme/` for render logic.

## Acceptance Criteria

- [x] `runie-tui/src/markdown.rs` renamed to `markdown_render.rs`; domain `markdown/` retains the pure name.
- [ ] `crates/runie-core/src/themes.rs` is renamed to `crates/runie-core/src/theme_tokens.rs`.
- [ ] All core callers of `runie_core::themes::BUILTIN_THEMES` are updated to `runie_core::theme_tokens::BUILTIN_THEMES`.
- [ ] `runie-tui/src/theme/loader.rs` continues to re-export the constant from the new core module path.
- [ ] `rg "^pub mod (diff|ui|ipc|markdown|themes|theme)" crates/runie-core/src/lib.rs crates/runie-tui/src/lib.rs` shows no name collision.
- [ ] All callers of renamed modules updated.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `theme_tokens_round_trip` — the renamed module still exports the same `BUILTIN_THEMES` list.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [ ] `theme_render_tests_pass_after_move` — TUI theme render tests pass after the module rename.

### Layer 4 — Smoke / Crash
- [ ] `smoke_no_module_name_collision` — a guardrail test scans `crates/runie-core/src/lib.rs` and `crates/runie-tui/src/lib.rs` `pub mod` declarations and fails on any shared name.

## Files touched

- `crates/runie-core/src/themes.rs` → `crates/runie-core/src/theme_tokens.rs`
- `crates/runie-core/src/lib.rs` (update `pub mod themes`)
- `crates/runie-core/src/settings/dialog.rs`
- `crates/runie-core/src/update/dialog/open.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-tui/src/theme/loader.rs`

## Notes

- The other module-name pairs (`diff`, `ui`, `ipc`) are already resolved and archived; this task covers only the `theme`/`themes` remainder.
- Do not move the token list into `runie-tui`; core uses it for validation and would need to depend on TUI.
- Rejected alternative: keeping `themes.rs` in core and accepting the semantic collision. The names are already distinct (`themes` vs `theme`), but a reader still has to know which crate owns which concept. Renaming to `theme_tokens.rs` makes the ownership obvious.
