# Use `pulldown-cmark-frontmatter` for declarative resource loading

**Status**: todo
**Milestone**: R2
**Category**: Core / State
**Priority**: P1

**Depends on**: unify-declarative-resource-loader
**Blocks**: none

## Description

`crates/runie-core/src/resource_loader.rs` implements its own YAML frontmatter scanning and markdown body extraction. `ctx7` found `pulldown-cmark-frontmatter`, a small, high-reputation crate that does exactly this on top of `pulldown-cmark`. Replacing the custom parser removes regex/string scanning, supports both YAML and TOML frontmatter naturally, and aligns with the SKILL.md/YAML frontmatter convention used by `thClaws` and `OpenFang`.

## Acceptance Criteria

- [ ] Add `pulldown-cmark-frontmatter` (and `serde_yaml`) to `runie-core` dependencies.
- [ ] `resource_loader.rs` uses `FrontmatterExtractor` to parse frontmatter and body; custom delimiter scanning is removed.
- [ ] Frontmatter is deserialized into a typed struct (or `HashMap<String, String>`) via `serde_yaml`/`toml` instead of manual line parsing.
- [ ] Subdirectory `SKILL.md` + flat `.md` precedence logic is preserved but expressed in terms of `walkdir`/`ignore`.
- [ ] Existing resource records loaded from `crates/runie-core/resources/` and project `.runie/skills/` produce identical `frontmatter`/`content` fields.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `resource_with_yaml_frontmatter` — parses title, version, and body.
- [ ] `resource_with_toml_frontmatter` — parses TOML code-block frontmatter (the format `pulldown-cmark-frontmatter` demonstrates).
- [ ] `resource_without_frontmatter` — body equals full file content, frontmatter empty.
- [ ] `subdir_skill_precedes_flat_file` — `foo/SKILL.md` wins over `foo.md`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] N/A.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/skills/mod.rs`
- `crates/runie-core/src/declarative/mod.rs`
- resource directories under `crates/runie-core/resources/` and `.runie/skills/`

## Notes

- `pulldown-cmark-frontmatter` uses the first Markdown code block as frontmatter by default. Ensure YAML frontmatter blocks are fenced with `---`/`---` or `yaml` language tags depending on what the crate expects; add a thin normalizer if Runie's existing resources use a different convention.
- `subagents/mod.rs:203-251` still implements its own frontmatter/body scanner; migrate subagent types to the same loader as part of this change.
- `OpenFang` and `thClaws` both treat `SKILL.md` as markdown with YAML frontmatter; aligning the loader means skills become portable across agents.
