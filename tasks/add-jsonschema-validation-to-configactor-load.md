# Add `jsonschema` validation to `ConfigActor` load path

**Status**: todo
**Milestone**: R6
**Category**: Configuration
**Priority": P2

**Depends on**: replace-config-validator-with-jsonschema
**Blocks**: route-cli-config-through-configactor

## Description

After the hand-written validator is replaced by `jsonschema`, wire validation into `RactorConfigActor::pre_start`/`reload` so invalid configs emit `Event::Error` instead of silently defaulting.

## Acceptance Criteria

- [ ] Validate loaded config against `config.schema.json` in `RactorConfigActor`.
- [ ] On validation failure, emit `Event::Error` with a typed message and keep the previous valid config or fail safe.
- [ ] Remove the old `validate.rs` call path.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_validation_rejects_unknown_field` — an unknown provider field fails validation.

### Layer 2 — Event Handling
- [ ] `config_actor_emits_error_on_invalid_config` — `RactorConfigActor` emits `Event::Error` on load.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/config/validate.rs`
- `crates/runie-core/src/events.rs`

## Notes

- Coordinate with `replace-config-validator-with-jsonschema.md`.
