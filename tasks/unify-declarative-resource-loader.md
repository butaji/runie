# Unify declarative resource loader

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: none
**Blocks**: use-pulldown-cmark-frontmatter-for-resource-loader

## Description

`crates/runie-core/src/skills/load.rs` and `crates/runie-core/src/declarative/loader.rs` both implement nearly identical directory-scanning and YAML-frontmatter logic for loading markdown resources. Unifying them removes ~150 duplicated lines and prevents the two loaders from drifting as the declarative DSL surface grows.

Current state as of Round 3:

- `skills/load.rs:9–182` scans a directory for `SKILL.md` subdirectories and flat `.md` files, extracts YAML frontmatter, resolves names, builds `Skill` records, and falls back to markdown-section parsing (`## Description`, `## Context`, `## Invocation`) when frontmatter is absent.
- `declarative/loader.rs:67–259` repeats the same pattern for `SkillDef`, but `parse_skill_md` returns `None` if frontmatter is missing.
- The bundled `resources/agents/`, `resources/skills/`, and `resources/commands/` directories referenced in docs and tests do not exist in the repo.

This task extracts the common scanning/frontmatter logic into a single loader module used by both `skills` and `declarative`. The richer `SkillDef` triggers and command-specific loading remain in `declarative`.

## Acceptance Criteria

- [x] Decide the frontmatter policy: the shared loader supports markdown-section fallback, and `skills/load.rs` uses it while `declarative/loader.rs` requires frontmatter.
- [x] A shared loader module exists: `crates/runie-core/src/resource_loader.rs` handles directory scanning, `SKILL.md` subdirectory precedence, flat `.md` files, YAML frontmatter extraction, name resolution, and markdown-section fallback.
- [x] `skills/load.rs` uses the shared loader to produce `Skill` records.
- [x] `declarative/loader.rs` uses the shared loader to produce `SkillDef` records.
- [x] Command loading (`load_commands_from_dir`) stays in `declarative` because commands are YAML, not markdown frontmatter.
- [x] No duplicated frontmatter parsing or directory-scanning logic remains between the two modules.
- [x] Update tests to use temporary directories (or seed bundled `resources/` directories if that is the intended product layout).
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `resource_loader_scans_skills` — `load_resources_from_dir` handles subdirs and flat files correctly.
- [x] `resource_loader_extracts_frontmatter` — YAML frontmatter is parsed correctly.
- [x] `skill_and_declarative_produce_same_names` — both loaders use the same underlying `load_resources_from_dir`.

### Layer 2 — Event Handling
- [x] N/A — loading is synchronous file I/O, not event dispatch.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Smoke / Crash
- [x] `load_all_skills_no_panic` — loading a temporary resource directory (or the bundled `resources/` tree if it exists) does not panic after the refactor.

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
- Once unified, replace the custom frontmatter/body scanner with `pulldown-cmark-frontmatter` + `serde_yaml` in `use-pulldown-cmark-frontmatter-for-resource-loader`. Use `walkdir`/`ignore` for directory traversal instead of manual `fs::read_dir` loops.
- `thClaws` and `OpenFang` both use YAML-frontmatter `SKILL.md` resources; aligning the loader makes Runie skills portable across agents.
- Out of scope: unifying the command YAML loader, fixing the declarative `Box::leak`/`CommandCategory` issues, or changing the resource file format.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

All validation gates confirmed:

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (internal loader unification).

**Verification (2026-07-01):** `load_resources_from_dir` exists in `runie-core/src/resource_loader.rs:27`. Both `declarative/loader.rs:70` and `skills/load.rs:19` call it. No duplicate frontmatter parsing remains.
