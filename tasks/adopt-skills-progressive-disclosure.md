# Adopt Skills Progressive Disclosure System

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement a full skill discovery, triggering, and progressive loading system inspired by Codex/codex-rs. Skills should support:

1. **Metadata layer** (~100 words) — always in context for matching
2. **Instructions layer** (<5k words) — loaded when skill triggers
3. **Bundled resources** — scripts, references, assets

The `SKILL.md` format with YAML frontmatter already exists (see `adopt-serde-yaml-skills`). Extend it to support:
- Skill discovery paths: `~/.runie/skills/`, `./.runie/skills/`, `~/.agents/skills/`
- Trigger mechanism: match user intent against `description` field
- Progressive loading: only load instruction content on trigger
- Skill directory structure: `skill-name/SKILL.md`, `skill-name/scripts/`, `skill-name/references/`

## Acceptance Criteria

- [x] Skill discovery scans multiple paths in priority order.
- [x] Skill matching uses description field for intent detection.
- [x] Progressive loading: metadata always loaded, instructions loaded on trigger.
- [x] Skill registry with `list_skills()`, `find_skill(name)`, `trigger_skill(intent)`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `skill_discovery_finds_skills_in_priority_paths` — local skills override global.
- [x] `skill_matching_selects_best_match` — intent matches best description.
- [x] `progressive_loading_only_loads_instructions_on_trigger` — lazy load verification.

### Layer 2 — Event Handling
- [x] `skill_triggers_on_intent_match` — skill loads when description matches.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [x] Smoke test loads skills from all paths.

## Files touched

- `crates/runie-core/src/skills.rs`
- `crates/runie-core/src/skill/` (new module)

## Notes

See `~/Code/agents/codex-rs/skills/` for reference implementation.
