# Support Peer-Standard Skill Layout: Subdirectories + YAML Frontmatter

**Status**: todo
**Milestone**: R2
**Category**: Configuration
**Priority**: P1

## Description

Runie's existing skills loader (`crates/runie-core/src/skills.rs`) reads flat
`~/.runie/skills/*.md` and `./.runie/skills/*.md` files and extracts
`## Description`, `## Context`, and `## Invocation` sections.

Research of `~/Code/agents` shows that modern agent CLIs uniformly use a nested
layout and YAML frontmatter:

- `.codex/skills/code-review-change-size/SKILL.md`
- `.agents/skills/write-tui/SKILL.md`
- `.claude/skills/*/SKILL.md`

```markdown
---
name: rust
description: Best practices for Rust code in this repo.
---

## Context
Always run `cargo clippy` before declaring code clean.
```

This task upgrades the loader to support that convention, while keeping the
existing flat-file behavior intact.

## Acceptance Criteria

- [ ] `crates/runie-core/src/skills.rs` `load_from_dir` scans immediate
  subdirectories for `<dir>/<name>/SKILL.md`.
- [ ] If both `<dir>/<name>.md` and `<dir>/<name>/SKILL.md` exist, the
  subdirectory version wins and the flat file is not loaded as a duplicate.
- [ ] The skill name is derived from the directory name for subdirectory skills,
  and from the file stem for flat files (existing behavior).
- [ ] Optional YAML frontmatter at the start of `SKILL.md` is parsed for
  `name` and `description`.
- [ ] Frontmatter `name` overrides the directory/file stem name.
- [ ] Frontmatter `description` overrides `## Description` if present; otherwise
  `## Description` is still used.
- [ ] Existing `## Description`, `## Context`, and `## Invocation` parsing
  continues to work for files without frontmatter.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `load_from_dir_prefers_subdirectory_skill` — given `rust.md` and
  `rust/SKILL.md`, only the subdirectory skill is returned.
- [ ] `subdirectory_skill_uses_dir_name` — loading `rust/SKILL.md` yields
  `name == "rust"`.
- [ ] `yaml_frontmatter_overrides_name_and_description` — frontmatter
  `name: foo` / `description: bar` wins over directory name and sections.
- [ ] `yaml_frontmatter_falls_back_to_sections` — frontmatter with only
  `description` uses `## Description` if `description` is absent.
- [ ] `flat_md_file_still_works` — existing `rust.md` layout continues to load.
- [ ] `build_skills_context_includes_subdir_skill_context` — context from a
  subdirectory skill is included in `build_skills_context`.

### Layer 2 — Event Handling
- [ ] `reload_all_reloads_subdirectory_skills` — `/reload` (or the equivalent
  `SystemEvent::ReloadAll`) picks up a newly created
  `.runie/skills/<name>/SKILL.md`.

### Layer 3 — Rendering
N/A (no UI change).

### Layer 4 — Smoke
N/A.

## Notes

**Why this matters:**
- A skill directory can carry side files (`examples/`, `tests/`, schemas).
- The nested layout is easier to browse, version, and share across projects.
- YAML frontmatter is the de-facto standard for skill metadata in Codex,
  Kimi Code, thClaws, OpenFang, and others.

**Backward compatibility:**
- Flat `~/.runie/skills/*.md` files continue to work unchanged.
- `## Description` / `## Context` / `## Invocation` remain valid.

**Parsing constraints:**
- Only recognize frontmatter when it starts at byte 0 with `---\n` and ends with
  a matching `---\n`.
- Use a small, dependency-free parser (regex or manual split); do not add a
  YAML crate just for two string fields.
- Unknown frontmatter keys are ignored.

**Files touched:**
- `crates/runie-core/src/skills.rs`
- `crates/runie-core/src/skills/tests.rs` (optional, if tests are split out)

**Out of scope:**
- Hot-reloading individual skills without a `/reload` command.
- Skill versioning or migration.
- Auto-discovery of skills in nested sub-subdirectories (one level only).
