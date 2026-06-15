# Adopt `serde_yml` for Skills YAML Frontmatter

**Status**: todo
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the manual YAML frontmatter string-splitting in `crates/runie-core/src/skills.rs` with proper YAML deserialization using `serde_yml`. Only the frontmatter extraction changes; the rest of skill loading stays custom.

## Acceptance Criteria

- [ ] `serde_yml` is added as a dependency.
- [ ] `skills.rs` deserializes frontmatter into a typed struct instead of splitting strings.
- [ ] Quoted strings, multiline values, and escapes are handled correctly.
- [ ] Existing skill files continue to load without changes.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `skill_frontmatter_parses_with_serde_yml` — frontmatter with quoted/multiline values parses.
- [ ] `skill_frontmatter_backward_compatible` — existing skill files still load.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/skills.rs`

## Notes

- `serde_yaml` is the older crate; `serde_yml` is the current community fork. Either is acceptable if it fits the dependency tree.
- See `docs/CRATE_DECISIONS.md`.
