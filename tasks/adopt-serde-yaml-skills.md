# Adopt `serde_yaml` for Skills YAML Frontmatter

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the manual YAML frontmatter string-splitting in `crates/runie-core/src/skills.rs`
with proper YAML deserialization using `serde_yaml`. Only the frontmatter extraction
changes; the rest of skill loading stays custom.

## What Was Done

- Added `serde_yaml = "0.9"` to workspace and runie-core dependencies.
- Replaced manual `for line in fm_text.lines()` parsing with `serde_yaml::from_str::<serde_yaml::Value>(fm_text)`.
- The `extract_frontmatter` function now handles all YAML value types:
  - Quoted strings (`"..."` and `'...'`)
  - Literal block scalars (`|` for multiline)
  - Folded block scalars (`>` for multiline)
  - Plain strings (no quotes)
  - Non-string values (lists, numbers) are gracefully ignored
- Frontmatter keys `name`, `description`, `context`, `invocation` are extracted from YAML;
  if missing, falls back to markdown sections as before.
- Added 7 new serde_yaml-specific tests covering quoted strings, multiline (literal/folded), mixed types.

## Acceptance Criteria

- [x] `serde_yaml` is added as a dependency.
- [x] `skills.rs` deserializes frontmatter into a typed struct instead of splitting strings.
- [x] Quoted strings, multiline values, and escapes are handled correctly.
- [x] Existing skill files continue to load without changes.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `skill_frontmatter_parses_with_serde_yml` — frontmatter with quoted/multiline values parses.
  - `serde_yaml_frontmatter_parses_quoted_strings` — double-quoted and single-quoted values.
  - `serde_yaml_frontmatter_parses_multiline_context` — literal block scalar (`|`).
  - `serde_yaml_frontmatter_parses_multiline_with_indentation` — folded block scalar (`>`).
  - `serde_yaml_frontmatter_ignores_non_string_values` — lists/numbers gracefully skipped.
  - `serde_yaml_frontmatter_no_frontmatter_returns_empty` — no `---` returns empty map.
  - `serde_yaml_frontmatter_empty_frontmatter_returns_empty` — empty `---` returns empty map.
  - `serde_yaml_frontmatter_single_quoted_values` — single-quoted YAML values.
- [x] `skill_frontmatter_backward_compatible` — existing skill files still load (59 skill tests pass).

## Files touched

- `Cargo.toml` — added `serde_yaml = "0.9"` to workspace deps
- `crates/runie-core/Cargo.toml` — added `serde_yaml.workspace = true`
- `crates/runie-core/src/skills.rs` — replaced manual YAML parsing with serde_yaml
