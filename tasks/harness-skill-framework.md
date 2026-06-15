# Harness Skill Framework

**Status**: todo
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
- [ ] `turn_start_emits_skill_event` — `on_turn_start` hook receives a `TurnStart` event.
- [ ] `tool_call_invokes_skill_hook` — `on_tool_call` hook receives pre/post tool events.

### Layer 3 — Rendering
- [ ] `skill_status_shows_in_diagnostics` — enabled/disabled skills appear in the diagnostics panel.

### Layer 4 — Smoke / Crash
- [ ] `smoke_skills_default_on` — fresh config starts with default skills enabled and binary runs.

## Files touched

- `crates/runie-core/src/harness_skills.rs` (new) — `HarnessSkill` trait, `SkillRegistry`, config types
- `crates/runie-core/src/lib.rs` — exports for harness_skills module
- `crates/runie-agent/src/turn.rs` — (pending) invoke skill hooks

## Notes

- Keep the trait sync for now; async can be added when needed for I/O.
- First-party skills are defined as empty structs that implement `HarnessSkill`.
- See `docs/adr/0022-harness-middleware-plugins.md` for motivation and hook design.
