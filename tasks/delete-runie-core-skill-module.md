# Delete dead `runie-core/src/skill/` module

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/skill/` (318 LOC, **singular** — distinct from the live `crates/runie-core/src/skills/`, plural) defines `pub struct SkillSummary`, `pub struct SkillRegistry` with `load_all() -> Self`, `list_skills`, `find_skill`, `trigger_skill`. `rg 'crate::skill::|runie_core::skill::' crates/` returns only the file itself; the only outward reference is `pub mod skill;` in `lib.rs:79`.

The file is a remnant of an abandoned "progressive disclosure" exploration. The live skill discovery lives in `skills/` (plural) and is consumed by `app_init.rs`, `dry_run.rs`, and `commands/dsl/handlers/system.rs`. The singular `skill/` module uses `crate::skills::Skill` for parsing but exposes a different type (`SkillSummary`) that nothing reads.

The name collision (`pub mod skill;` + `pub mod skills;`) is also a source of reader confusion — only the plural one is alive.

## Acceptance Criteria

- [x] `crates/runie-core/src/skill/` directory deleted (mod.rs + contents).
- [x] `pub mod skill;` removed from `crates/runie-core/src/lib.rs`.
- [x] `rg 'crate::skill\b|runie_core::skill\b' crates/` returns zero hits outside `tasks/`.
- [x] `rg "use crate::skills\b" crates/` is unchanged (live `skills/` untouched).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — module deletion.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_skill_singular_module_gone` — workspace builds with `skill/` removed.
- [x] `smoke_skills_plural_still_loads` — `runie-core` skill discovery still works end-to-end (existing tests in `commands/tests/skills.rs` pass).

## Files touched

- `crates/runie-core/src/skill/` (entire directory deleted)
- `crates/runie-core/src/lib.rs` (removed `pub mod skill;`)

## Notes

Distinct from the live `skills/` (plural) module and from `harness_skills/` (different concept: harness interceptors on the agent turn).
