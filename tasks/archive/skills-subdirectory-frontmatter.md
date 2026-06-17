# Support Peer-Standard Skill Layout: Subdirectories + YAML Frontmatter

**Status**: done
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

This task upgrades the loader to support that convention, while keeping the
existing flat-file behavior intact.

## Acceptance Criteria

- [x] `crates/runie-core/src/skills.rs` `load_from_dir` scans immediate
  subdirectories for `<dir>/<name>/SKILL.md`.
- [x] If both `<dir>/<name>.md` and `<dir>/<name>/SKILL.md` exist, the
  subdirectory version wins and the flat file is not loaded as a duplicate.
- [x] The skill name is derived from the directory name for subdirectory skills,
  and from the file stem for flat files (existing behavior).
- [x] Optional YAML frontmatter at the start of `SKILL.md` is parsed for
  `name` and `description`.
- [x] Frontmatter `name` overrides the directory/file stem name.
- [x] Frontmatter `description` overrides `## Description` if present; otherwise
  `## Description` is still used.
- [x] Existing `## Description`, `## Context`, and `## Invocation` parsing
  continues to work for files without frontmatter.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `subdirectory_skill_loads` — subdirectory skill with SKILL.md loads correctly.
- [x] `subdirectory_prefers_over_flat_file` — subdirectory wins over flat file with same name.
- [x] `yaml_frontmatter_overrides_name_and_description` — frontmatter wins over sections.
- [x] `yaml_frontmatter_falls_back_to_sections` — partial frontmatter falls back correctly.
- [x] `flat_md_file_still_works` — existing layout continues to work.
- [x] `build_skills_context_includes_subdir_skill` — context from subdir skills included.

## Files touched

- `crates/runie-core/src/skills.rs` — added subdirectory and frontmatter support
