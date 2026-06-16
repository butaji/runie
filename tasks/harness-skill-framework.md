# Harness Skill Framework

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: adopt-or-remove-actor-framework
**Blocks**: harness-skill-hashline-edit, harness-skill-verification-loop, harness-skill-startup-context, harness-skill-loop-detector, harness-skill-tool-schema-enricher

## Description

Runie already defines a Skill as an event-bus interceptor in `docs/CONTEXT.md`. This task creates the concrete framework that makes Skills default-on, configurable, and togglable. The framework exposes lifecycle hooks (`on_turn_start`, `on_tool_call`, `on_turn_end`) and loads per-skill config from `~/.runie/config.toml` under `[harness.skills]`.

## Acceptance Criteria

- [x] A `HarnessSkill` trait exists in `runie-core` with hooks for `on_turn_start`, `on_tool_call`, and `on_turn_end`.
- [x] A `SkillRegistry` manages skills, applies default-on config, and lets users disable or reconfigure each skill.
- [x] Config schema under `[harness]` and `[harness.skills.<name>]` is defined and deserializable from TOML.
- [x] Skills can be ordered; conflicts between skills are resolved by explicit precedence.
- [x] The agent turn in `runie-agent/src/turn.rs` invokes skill hooks without hard-coding skill logic.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `skill_registry_loads_defaults` — default skills are enabled when config is absent.
- [x] `skill_registry_respects_disabled_flag` — setting `enabled = false` removes the skill from active hooks.
- [x] `skill_order_is_deterministic` — registry returns skills in configured order.
- [x] `config_deserializes_from_toml` — harness config parses correctly.

### Layer 2 — Event Handling
- [x] `on_turn_start_all_continue` — skills return Continue by default.
- [x] `on_turn_start_first_abort_wins` — abort short-circuits.

### Layer 3 — Rendering
N/A (diagnostics panel deferred).

### Layer 4 — Smoke / Crash
N/A (smoke test deferred).

## Files touched

- `crates/runie-core/src/harness_skills.rs` — `HarnessSkill` trait, `SkillRegistry`, config types
- `crates/runie-core/src/lib.rs` — exports for harness_skills module
- `crates/runie-agent/src/turn.rs` — invokes skill hooks

## Notes

- Core framework implemented with sync trait.
- Individual skills (hashline-edit, verification-loop, etc.) are separate tasks.
