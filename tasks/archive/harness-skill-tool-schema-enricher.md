# Harness Skill: Tool Schema Enricher

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: harness-skill-framework, tool-registry-trait
**Blocks**: (none)

## Description

Add concrete `examples` arrays to tool schemas before they are sent to the LLM. Research shows that better tool descriptions and examples can drop tool-usage failure rates from 30–50% to ~1%.

## Acceptance Criteria

- [ ] Skill hooks `on_turn_start` to enrich tool schemas with examples before they reach the LLM.
- [ ] Examples are defined per tool and are stable across model providers.
- [ ] Enrichment can be disabled globally or per tool under `[harness.skills.tool_schema_enricher]`.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `schemas_contain_examples_when_enabled` — enabled skill adds examples to built-in tool schemas.
- [ ] `schemas_unchanged_when_disabled` — disabled skill leaves schemas as-is.

### Layer 2 — Event Handling
N/A — schema enrichment is a turn-start transformation.

### Layer 3 — Rendering
N/A — no direct UI change.

### Layer 4 — Smoke / Crash
N/A — covered by Layer 1.

## Files touched

- `crates/runie-core/src/skills/tool_schema_enricher.rs`
- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-agent/src/turn.rs`

## Notes

- Examples should be short and realistic. One example per major parameter pattern is enough.
- This skill pairs naturally with `harness-skill-hashline-edit` when Hashline is enabled.
- See `docs/adr/0022-harness-middleware-plugins.md`.
