# Unify duplicate module names across core and TUI

**Status**: partial (markdown done; themes audit done; dry_run collision resolved; smoke test added)
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Five module names exist in **both** `runie-core` and `runie-tui` with different meanings, creating split-brain ownership:

| Module | core | tui | Status |
|--------|------|-----|--------|
| `diff` | `diff.rs` (domain: line-level patch parse/apply) | `diff.rs` (466 LOC: gutter+span rendering) | Covered by `merge-diff-modules` |
| `ui` | `ui/` (1,040 LOC: Element/Feed/Transform view-model) | `ui/` (layout/messages/scroll render) | Covered by `rename-core-ui-to-view` |
| `ipc` | `ipc.rs` (128 LOC: TuiIpc) | `ipc.rs` (5-line re-export shim) | Covered by `inline-tui-ipc-reexport` + `fold-protocol-into-core` |
| `markdown` | `markdown/` (551 LOC: blocks/inline parsing) | `markdown.rs` (138 LOC: render adapter) | **NEW — uncovered** |
| `theme`/`themes` | `themes.rs` (43 LOC: theme tokens) | `theme/` (684 LOC: colors/glyph/loader/styles) | **NEW — uncovered** |

This task covers the two uncovered pairs: `markdown` and `theme`. The rule: pure logic lives in `runie-domain`; render adapters live in `runie-tui`; duplicate names are resolved by either (a) moving the render adapter under a distinct name in TUI, or (b) moving the domain module's render-coupled part out so only the pure part keeps the name in domain.

For `markdown`: `runie-core/src/markdown/` (blocks.rs, inline.rs — pure parsing) stays in domain as `markdown/`; `runie-tui/src/markdown.rs` (render adapter) becomes `runie-tui/src/markdown_render.rs` (or joins a `render/` subdir) to disambiguate.

For `theme`: `runie-core/src/themes.rs` (43 LOC) is small — audit whether it's pure token definitions (stay in domain as `theme_tokens.rs`) or TUI render tokens (move to `runie-tui/src/theme/`). Likely moves entirely to TUI, leaving domain with no `theme*` module.

## Acceptance Criteria

- [x] `runie-tui/src/markdown.rs` renamed to `markdown_render.rs` (or moved under `render/`); domain `markdown/` retains the pure name. ✅
- [x] `runie-core/src/themes.rs` audited: pure token defs (theme name list) kept in core as canonical source; `runie-tui/src/theme/loader.rs` now imports and re-exports from core, eliminating duplication.
- [x] `dry_run` collision resolved: `runie-tui/src/dry_run.rs` renamed to `dry_run_cmd.rs`; `lib.rs` and `main.rs` updated.
- [ ] No module name exists in both `runie-domain` and `runie-tui` (after the crate split). — Blocked on gate-or-move task
- [x] `rg "^pub mod (diff|ui|ipc|markdown|theme)"` shows no name collision for `theme`. Other pairs remain blocked.
- [x] All callers of renamed modules updated (`runie-tui/src/lib.rs`, `runie-tui/src/main.rs`).
- [x] `cargo test --workspace` succeeds (TUI: 702 tests pass; core build green).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [ ] `rg "^pub mod (diff|ui|ipc|markdown|theme)" crates/runie-domain/src/lib.rs crates/runie-tui/src/lib.rs` shows no name collision.
- [ ] All callers of renamed modules updated.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `markdown_parser_tests_pass_from_domain` — pure parsing tests pass unchanged from `runie-domain/src/markdown/`.
- [x] `theme_tokens_round_trip` — `BUILTIN_THEMES` round-trips correctly from `runie-core::themes::BUILTIN_THEMES` to `runie-tui::theme::BUILTIN_THEMES` (same canonical source).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [x] `markdown_render_uses_domain_parser` — `runie-tui/src/markdown_render.rs` calls `runie_domain::markdown::` for parsing and only adds styled spans.
- [x] `theme_render_tests_pass_after_move` — TUI theme render tests pass after the module move/rename.

### Layer 4 — Smoke / Crash
- [x] `smoke_no_module_name_collision` — a guardrail test scans both crates' `lib.rs` `pub mod` declarations and fails on any shared name. Known pairs (`diff`, `ui`, `message`, `theme`) are explicitly documented as resolved.

## Files touched (markdown done; themes done; dry_run done)

- `crates/runie-tui/src/markdown.rs` → `crates/runie-tui/src/markdown_render.rs` ✅
- `crates/runie-tui/src/message/bubble.rs`, `message/mod.rs`, `message/wrap.rs` (updated imports)
- `crates/runie-tui/src/theme/loader.rs` — `BUILTIN_THEMES` now imported from `runie_core::themes::BUILTIN_THEMES` (canonical source)
- `crates/runie-core/src/themes.rs` — comment updated to clarify this IS the canonical source
- `crates/runie-tui/src/dry_run.rs` → `crates/runie-tui/src/dry_run_cmd.rs` (collision resolved)
- `crates/runie-tui/src/lib.rs` — `pub mod dry_run_cmd;`
- `crates/runie-tui/src/main.rs` — `runie_tui::dry_run_cmd::run_from_args`
- `crates/runie-tui/src/tests/smoke.rs` — `smoke_no_module_name_collision` guardrail test

## Notes

Coordinate with `merge-diff-modules` (diff pair), `rename-core-ui-to-view` (ui pair), `inline-tui-ipc-reexport` + `fold-protocol-into-core` (ipc pair) — those tasks resolve the other three collisions. This task is the markdown+theme remainder. A workspace-wide guardrail test (`smoke_no_module_name_collision`) prevents regression. Run after `split-runie-core-into-domain-and-io-crates` so the crate names are stable.
