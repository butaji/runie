# Adopt Layered Configuration with JSON Schema

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Enhance configuration system with:

1. **JSON Schema validation** — validate config against schema at load time
2. **Multi-provider support** — configure multiple API providers with fallback
3. **Layered sources** — env vars > local config > global config > defaults
4. **Schema generation** — `just write-config-schema` regenerates `config.schema.json`

Reference: `~/Code/agents/codex-rs/core/src/config/` and `justfile` schema targets.

## Acceptance Criteria

- [ ] `config.schema.json` generated from Config struct.
- [ ] Config validation on load with clear error messages.
- [ ] Multiple providers configurable with model selection per provider.
- [ ] Layered config sources: defaults → global → local → env.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `config_validation_rejects_invalid_json` — schema validation works.
- [ ] `layered_config_env_overrides_file` — precedence correct.
- [ ] `multi_provider_falls_back_to_second` — provider fallback works.

### Layer 2 — Event Handling
- [ ] `config_reload_applies_changes` — file watcher triggers reload.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test with invalid config shows helpful error.

## Files touched

- `crates/runie-core/src/config/` (existing, enhanced)
- `config.schema.json` (generated)

## Notes

Leverages existing `adopt-notify-config-watcher` work.
