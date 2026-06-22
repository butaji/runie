# Decide harness_skills framework fate

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/harness_skills/` (~885 LOC) defines `HarnessSkill` trait, `SkillRegistry`, `HarnessConfig`, and 5 skill impls (`HashlineEditSkill`, `LoopDetectorSkill`, `StartupContextSkill`, `ToolSchemaEnricherSkill`, `VerificationLoopSkill`). The production entry `run_agent_turn` hardcodes `None` for skills (`turn.rs:27`); every hook call is guarded by `if let Some(skills) = skills`. The 5 skills are never instantiated in production — `with_skills` is called only from `tests/turn.rs` and `tests/minimax_turn.rs`. Either wire a real `SkillRegistry` into the production turn or delete the framework.

## Acceptance Criteria

- [ ] Decision made and executed: EITHER
  - (a) a production `SkillRegistry` is constructed and passed to `run_agent_turn` from `AgentActor`/`subagent`, with at least one skill enabled by config; OR
  - (b) `harness_skills/` deleted, `run_agent_turn_with_skills` collapsed back into `run_agent_turn`, and test callers migrated to the simpler signature.
- [ ] No `if let Some(skills) = skills` guard remains if option (b) is chosen.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `skill_registry_construction_or_absent` — depending on option: either a registry builds from config, or no `SkillRegistry` type exists.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_turn_with_or_without_skills` — agent turn completes regardless of chosen option.

## Files touched

- `crates/runie-core/src/harness_skills/` (delete or wire)
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/subagent.rs`
- `crates/runie-agent/src/tests/turn.rs`
- `crates/runie-agent/tests/minimax_turn.rs`

## Notes

Recommended: option (b) delete, unless a concrete user-facing skill is shipping now. YAGNI — 885 LOC of unused abstraction. The Layer-4 test infrastructure in `tests/turn.rs` can keep a local mock-skill helper if needed.
