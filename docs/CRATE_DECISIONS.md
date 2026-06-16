# Crate Replacement Decisions

This document records which third-party crates replace custom Runie code, which custom code remains, and why.

## Status Legend

- **Adopt** — use the crate and remove/replace the corresponding custom code.
- **Keep** — custom code is project-specific or currently better than available crates.
- **Defer** — revisit later when requirements or crate maturity change.

## Adopted

| Area | Custom code | Crate | Context7 ID / crates.io | Decision |
|------|-------------|-------|-------------------------|----------|
| Markdown parsing | (was custom) | `pulldown-cmark` | `/pulldown-cmark/pulldown-cmark` | Adopted (from prior audit). |
| Syntax highlighting | (was custom) | `syntect` | `/trishume/syntect` | Adopted (from prior audit). |
| Agent diff generation | (was custom) | `similar` | `/mitsuhiko/similar` | Adopted (from prior audit). |
| Token counting | (was custom) | `tiktoken-rs` | crates.io | Adopted (from prior audit). |
| Config file watcher | `config_reload/watcher.rs` polling loop | `notify` | `/notify-rs/notify` | Adopt. Removes 2 s polling. |
| Clipboard image I/O | `clipboard_image.rs` subprocess hacks | `arboard` | `/websites/rs_arboard` | Adopt. Cross-platform, no temp files. |
| TUI diff parsing | `runie-tui/src/diff.rs` hand parser | `patch` | crates.io `patch 0.7.0` | Adopt. Fixes line-number bug. |
| File search & traversal | `tool/grep.rs`, `tool/find.rs`, `list_dir.rs`, `@` picker | `fff-search` | `/dmtrkovalenko/fff` | Adopt. Native, typed, frecency-aware. |
| Skills YAML frontmatter | `skills.rs` string splitter | `serde_yml` | `/websites/rs_crate_serde_yml` | Adopt. Correct YAML parsing. |
| Theme color helpers | `theme.rs` RGB darken/blend | `palette` | `/ogeon/palette` | Adopt. Color-space correct. |
| Non-file fuzzy matching | `fuzzy.rs` for panels/selectors | `nucleo-matcher` | crates.io | Adopt. Better Unicode/ranking. |
| Word wrapping | `layout.rs` / `display_width.rs` | `textwrap` | `/websites/rs_textwrap_0_16_2` | Adopt with parity tests. |
| Session persistence | `session_store.rs` JSONL + index | `redb` | `/cberner/redb` | Adopt. ACID + indexing. |

## Rejected / Absorbed

| Area | Custom code | Proposed crate | Reason |
|------|-------------|----------------|--------|
| Directory traversal | `file_refs.rs`, `path_complete.rs` | `ignore` + `globset` | Absorbed by `fff-search`. |
| File fuzzy matching | `@` picker scoring | `nucleo-matcher` | Absorbed by `fff-search` fuzzy path search. |

## Kept Custom

| Area | Reason |
|------|--------|
| Text-input widget (`update/input_text.rs`, `ui/input.rs`) | High behavior/test risk; `tui-input` does not fit vim mode, `@`-refs, and completions. |
| Command palette DSL | Runie-specific interactive builder tied to the event model. |
| Dialog & form DSL | Tied to Runie’s event model and Ratatui layout. |
| Update dispatcher | Large `match` router; state-machine crates add indirection. |
| Actor runtime / EventBus | Lightweight and sufficient for current needs. |
| Streaming buffer | Project-specific incremental “stable vs tail” behavior; no drop-in crate. |

## Pending Cleanup

| Area | Status | Task |
|------|--------|------|
| `runie-term-archive` crate | Delete after porting unique tests | `tasks/delete-runie-term-archive.md` |
| `runie-core` monolith | Extract tool/update/dialog/ui logic to `runie-agent`/`runie-engine`/`runie-tui` | `tasks/extract-core-monolith.md` |

## References

- Prior audit: `tasks/archive/crate-replacement-audit.md`
- FFF integration: `docs/adr/0023-fff-search-integration.md`
