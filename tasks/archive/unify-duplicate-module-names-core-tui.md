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
| `theme`/`themes` | `themes.rs` (token list) | `theme/` (colors/glyph/loader/styles) | **Resolved in code** — core renamed to `theme_tokens.rs`; guardrail ignore list still stale |

Current state as of this review:

- `crates/runie-core/src/themes.rs` has been renamed to `crates/runie-core/src/theme_tokens.rs` in the working tree.
- Core callers (`settings/dialog.rs:290`, `update/dialog/open.rs:116`, `update/system.rs:121`) and the TUI loader (`crates/runie-tui/src/theme/loader.rs:4`) have been updated to `runie_core::theme_tokens::BUILTIN_THEMES`.
- The guardrail test in `crates/runie-tui/src/tests/smoke.rs` still includes `"theme"` in its ignore list. Since the collision no longer exists, that ignore entry is stale and should be removed.

## Acceptance Criteria

- [x] `runie-tui/src/markdown.rs` renamed to `markdown_render.rs`; domain `markdown/` retains the pure name.
- [x] `crates/runie-core/src/themes.rs` renamed to `crates/runie-core/src/theme_tokens.rs`.
- [x] All core callers of `runie_core::themes::BUILTIN_THEMES` updated to `runie_core::theme_tokens::BUILTIN_THEMES`.
- [ ] Remove the stale `"theme"` entry from the `ignored` array in `crates/runie-tui/src/tests/smoke.rs`.
- [ ] `rg "^pub mod (diff|ui|ipc|markdown|themes|theme)" crates/runie-core/src/lib.rs crates/runie-tui/src/lib.rs` shows no name collision.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `theme_tokens_round_trip` — the renamed module still exports the same `BUILTIN_THEMES` list.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [x] `theme_render_tests_pass_after_move` — TUI theme render tests pass after the module rename.

### Layer 4 — Smoke / Crash
- [ ] `smoke_no_module_name_collision` — a guardrail test scans `crates/runie-core/src/lib.rs` and `crates/runie-tui/src/lib.rs` `pub mod` declarations and fails on any shared name, with no stale ignores.

## Files touched

- `crates/runie-core/src/themes.rs` → `crates/runie-core/src/theme_tokens.rs` ✅
- `crates/runie-core/src/lib.rs`
- `crates/runie-core/src/settings/dialog.rs`
- `crates/runie-core/src/update/dialog/open.rs`
- `crates/runie-core/src/update/system.rs`
- `crates/runie-tui/src/theme/loader.rs`
- `crates/runie-tui/src/tests/smoke.rs` (remove stale ignore)

## Notes

- The other module-name pairs (`diff`, `ui`, `ipc`) are already resolved and archived; this task covers only the `markdown` + `theme` remainder.
- Do not move the token list into `runie-tui`; core uses it for validation and would need to depend on TUI.
- Rejected alternative: keeping `themes.rs` in core and accepting the semantic collision. Renaming to `theme_tokens.rs` makes the ownership obvious.
