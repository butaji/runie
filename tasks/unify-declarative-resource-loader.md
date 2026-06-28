# Unify declarative resource loader

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/skills/load.rs` and `crates/runie-core/src/declarative/loader.rs` both implement nearly identical directory-scanning and YAML-frontmatter logic for loading markdown resources. Unifying them removes ~150 duplicated lines and prevents the two loaders from drifting as the declarative DSL surface grows.

Current state as of Round 3:

- `skills/load.rs:9–182` scans a directory for `SKILL.md` subdirectories and flat `.md` files, extracts YAML frontmatter, resolves names, builds `Skill` records, and falls back to markdown-section parsing (`## Description`, `## Context`, `## Invocation`) when frontmatter is absent.
- `declarative/loader.rs:67–259` repeats the same pattern for `SkillDef`, but `parse_skill_md` returns `None` if frontmatter is missing.
- The bundled `resources/agents/`, `resources/skills/`, and `resources/commands/` directories referenced in docs and tests do not exist in the repo.

This task extracts the common scanning/frontmatter logic into a single loader module used by both `skills` and `declarative`. The richer `SkillDef` triggers and command-specific loading remain in `declarative`.

## Acceptance Criteria

- [ ] Decide the frontmatter policy: either the shared loader always requires frontmatter, or it supports the markdown-section fallback consistently for both `skills` and `declarative`.
- [ ] A shared loader module exists (e.g., `crates/runie-core/src/resource_loader.rs` or under `declarative/`) that handles directory scanning, `SKILL.md` subdirectory precedence, flat `.md` files, YAML frontmatter extraction, name resolution, and the agreed-on fallback behavior.
- [ ] `skills/load.rs` uses the shared loader to produce `Skill` records.
- [ ] `declarative/loader.rs` uses the shared loader to produce `SkillDef` records.
- [ ] Command loading (`load_commands_from_dir`) stays in `declarative` because commands are YAML, not markdown frontmatter.
- [ ] No duplicated frontmatter parsing or directory-scanning logic remains between the two modules.
- [ ] Update tests to use temporary directories (or seed bundled `resources/` directories if that is the intended product layout).
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `resource_loader_scans_skills` — a temporary directory with both `SKILL.md` subdirs and flat `.md` files is loaded correctly, with subdirs taking precedence.
- [ ] `resource_loader_extracts_frontmatter` — YAML frontmatter fields (`name`, `description`, `context`, `invocation`) are parsed and returned as a map.
- [ ] `skill_and_declarative_produce_same_names` — given the same input directory, `skills::load_from_dir` and `declarative::load_skills_from_dir` produce the same skill names and descriptions.

### Layer 2 — Event Handling
- [ ] N/A — loading is synchronous file I/O, not event dispatch.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `load_all_skills_no_panic` — loading a temporary resource directory (or the bundled `resources/` tree if it exists) does not panic after the refactor.

## Files touched

- New `crates/runie-core/src/resource_loader.rs` (or `crates/runie-core/src/declarative/resource_loader.rs`)
- `crates/runie-core/src/skills/load.rs`
- `crates/runie-core/src/declarative/loader.rs`
- `crates/runie-core/src/lib.rs` (export if needed)

## Notes

- Keep the public API of `skills::load_all`/`load_from_dir` stable so external callers do not change.
- The shared loader should not know about `Skill` vs `SkillDef`; it should return generic frontmatter + file-path records that the two callers map into their own types.
- If the product intends to ship bundled `resources/` directories, seed them in a separate commit before or alongside this task. Otherwise, update tasks/docs that assume they exist.
- This is a high-Pareto change: small, safe, and removes a clear duplication hotspot that will grow as more declarative resource types are added.
- Out of scope: unifying the command YAML loader, fixing the declarative `Box::leak`/`CommandCategory` issues, or changing the resource file format.
