# Relocate loose _tests.rs files into module tests/ subdirs

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: consolidate-dual-path-modules
**Blocks**: none

## Description

Eight `*_tests.rs` files sit beside the module they test, using the `_tests` suffix with either `#[path = "..."]` includes or `mod X_tests;` declarations. This breaks the two conventional layouts (inline `#[cfg(test)] mod tests { ... }` or `foo/tests.rs` inside a `foo/` dir) and makes test discovery harder.

| File | LOC | Wired via | Notes |
|------|-----|-----------|-------|
| `crates/runie-core/src/file_refs_lookup_tests.rs` | 15 | `mod file_refs_lookup_tests;` in `lib.rs:98` | At src root — most unusual |
| `crates/runie-core/src/tool_parser_tests.rs` | 106 | `#[path = "..."] mod tests;` from `tool_parser.rs:360` | `#[path]` workaround |
| `crates/runie-core/src/update/dialog/form_tests.rs` | 148 | `#[path = "..."] mod form_tests;` from `form.rs:359` | `#[path]` workaround |
| `crates/runie-core/src/login_flow/state_tests.rs` | 140 | `mod state_tests;` from `login_flow/mod.rs` | Sits next to `state.rs` — could be `state/tests.rs` if `state.rs` becomes `state/mod.rs` |
| `crates/runie-core/src/event/variants_tests.rs` | 220 | `mod variants_tests;` from `event/mod.rs:48` | Tracked separately by `simplify-event-module-layout` |
| `crates/runie-agent/src/truncate_tests.rs` | 131 | `mod truncate_tests;` from `lib.rs` | Could be `truncate/tests.rs` |
| `crates/runie-provider/src/config_tests.rs` | 217 | `mod config_tests;` from `lib.rs` | Could be `config/tests.rs` |
| `crates/runie-tui/src/theme_tests.rs` | — | `mod theme_tests;` from `lib.rs` | Could be `theme/tests.rs` |

The `#[path = "..."]` workaround (tool_parser, form) exists because Rust 2018 disallows `foo.rs` + `foo/tests.rs` simultaneously — converting `foo.rs` → `foo/mod.rs` (see `consolidate-dual-path-modules`) lets the test file move into `foo/tests.rs` and the `#[path]` attribute drop.

## Acceptance Criteria

- [ ] `file_refs_lookup_tests.rs` either inlined into `file_refs.rs` as `#[cfg(test)] mod tests { ... }` or moved into a `file_refs/tests.rs` if `file_refs.rs` is converted to `file_refs/mod.rs`.
- [ ] `tool_parser_tests.rs` moved into `tool_parser/tests.rs` after `consolidate-dual-path-modules` converts `tool_parser.rs` → `tool_parser/mod.rs`; `#[path]` attribute removed.
- [ ] `update/dialog/form_tests.rs` moved into `update/dialog/form/tests.rs` after `form.rs` → `form/mod.rs`; `#[path]` removed. (Or inline.)
- [ ] `login_flow/state_tests.rs` moved into `login_flow/state/tests.rs` if `state.rs` → `state/mod.rs`, OR renamed to `login_flow/state_tests.rs` → inline. Pick one.
- [ ] `event/variants_tests.rs` handled by `simplify-event-module-layout` (not this task).
- [ ] `runie-agent/src/truncate_tests.rs` moved into `truncate/tests.rs` if `truncate.rs` → `truncate/mod.rs`, OR inlined.
- [ ] `runie-provider/src/config_tests.rs` moved into `config/tests.rs` if `config.rs` → `config/mod.rs`, OR inlined.
- [ ] `runie-tui/src/theme_tests.rs` moved into `theme/tests.rs` (theme is already a dir).
- [ ] No `*_tests.rs` file remains at a crate src root (`crates/*/src/*_tests.rs` outside of a `tests/` subdir).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (test counts unchanged).

## Tests

### Layer 1 — State/Logic
- N/A — file relocation, no logic change.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_test_count_unchanged_after_relocation` — `cargo test --workspace --no-fail-fast` reports the same number of tests as before the move (the test bodies are unchanged, only file locations move).
- [ ] `smoke_no_path_attribute_workarounds_remain` — `rg "#\[path = " crates/` returns zero hits (the `#[path]` workarounds are gone once dual-path layout is fixed).

## Files touched

- `crates/runie-core/src/file_refs_lookup_tests.rs` (move or inline)
- `crates/runie-core/src/tool_parser_tests.rs` → `tool_parser/tests.rs`
- `crates/runie-core/src/update/dialog/form_tests.rs` → `update/dialog/form/tests.rs`
- `crates/runie-core/src/login_flow/state_tests.rs` (move)
- `crates/runie-agent/src/truncate_tests.rs` (move or inline)
- `crates/runie-provider/src/config_tests.rs` (move or inline)
- `crates/runie-tui/src/theme_tests.rs` → `theme/tests.rs`
- Corresponding `lib.rs` / `mod.rs` declarations (update `mod` lines, drop `#[path]`)

## Notes

Depends on `consolidate-dual-path-modules` so the `#[path]` workaround files (`tool_parser_tests`, `form_tests`) can move into their module dirs after the dual-path consolidation. The `event/variants_tests.rs` move is owned by `simplify-event-module-layout` and excluded here to avoid double-listing. Use `git mv` to preserve history.
