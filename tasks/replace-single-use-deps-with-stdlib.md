# Replace single-use deps with stdlib where trivial

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: remove-unused-cargo-deps
**Blocks**: none

## Description

Several workspace deps are used in only 1-3 files for functionality the stdlib or a few lines of code can provide. `remove-unused-cargo-deps` targets truly unused deps; this task targets underused deps where the stdlib replacement is trivial and the dep adds build weight. Candidates, ranked by replacement ease:

| Dep | Files | Replacement | Effort |
|-----|-------|-------------|--------|
| `parking_lot` | 3 | `std::sync::Mutex`/`RwLock` | Low — drop-in, std locks are fine at this scale |
| `palette` | 2 | ~20 lines of std RGB math (sRGB → linear, blend) | Medium — verify color tests pass |
| `serde_yaml` | 1 | `toml` (skills frontmatter is constrained) or 30-line hand parser | Medium — verify frontmatter edge cases |

Keep (justified by feature): `redb` (ACID store), `textwrap` (Unicode line breaking), `nucleo-matcher` (fuzzy ranking), `pulldown-cmark` (markdown), `similar` (diff), `tiktoken-rs` (tokenization), `fff-search` (native search), `notify` (fs watcher), `arboard` (clipboard), `patch` (diff parsing).

## Acceptance Criteria

- [ ] `parking_lot` removed from `[workspace.dependencies]` and all crate `Cargo.toml`s; replaced with `std::sync::Mutex`/`RwLock`.
- [ ] `palette` removed; color blend/darken helpers reimplemented in ~20 lines of std f32 math in `crates/runie-tui/src/` (or `runie-core/src/themes.rs`).
- [ ] `serde_yaml` removed; skills frontmatter parsed with `toml` (if frontmatter is TOML-compatible) or a 30-line parser.
- [ ] All color tests (`adopt-palette-theme-colors` suite, 379 TUI tests) pass after `palette` removal.
- [ ] All skill tests (59 tests) pass after `serde_yaml` removal.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `std_mutex_replaces_parking_lot` — all 3 call sites compile with `std::sync::Mutex`; no deadlock or poisoning in tests.
- [ ] `color_blend_matches_palette_output` — std RGB blend produces the same RGB values as `palette` did (within 1-bit tolerance).
- [ ] `frontmatter_parses_all_skill_files` — the 59 skill tests pass with the new parser; quoted strings, multiline blocks, non-string values all handled.

### Layer 2 — Event Handling
- [ ] N/A — dep replacement, no event logic.

### Layer 3 — Rendering
- [ ] `theme_renders_identically` — snapshot tests for themes produce the same frames after `palette` removal.

### Layer 4 — Smoke / Crash
- [ ] `cargo build --workspace` green with fewer transitive deps.
- [ ] `cargo test --workspace` green.

## Files touched

- `Cargo.toml` (root) — remove `parking_lot`, `palette`, `serde_yaml` from `[workspace.dependencies]`
- 3 files using `parking_lot` — replace with `std::sync`
- 2 files using `palette` — replace with std RGB math
- 1 file using `serde_yaml` — replace with `toml` or hand parser
- `Cargo.lock` — regenerated

## Notes

Depends on `remove-unused-cargo-deps` (which removes truly unused deps first, so this task only touches underused ones). `parking_lot` is the safest win — std `Mutex` is identical ergonomics at this scale. `palette` is the riskiest — verify the 379 TUI color tests pass before committing. `serde_yaml` is medium — skills frontmatter is the only YAML in the codebase; if it's simple enough, `toml` may parse it, or a 30-line parser handles the constrained subset. Rejected alternative: keep all 3 "for correctness" — rejected because std `Mutex` is correct, RGB math is well-known, and YAML-for-frontmatter is overkill.
