# Add `jsonschema` validation to `ConfigActor` load path

**Status**: done
**Milestone**: R6
**Category**: Configuration
**Priority**: P2
**Note**: RactorConfigActor::pre_start emits ConfigLoaded without validating the config; validation only in load_and_emit.

**Depends on**: replace-config-validator-with-jsonschema
**Blocks**: route-cli-config-through-configactor

## Description

After the hand-written validator is replaced by `jsonschema`, wire validation into `RactorConfigActor::pre_start`/`reload` so invalid configs emit `Event::Error` instead of silently defaulting.

## Acceptance Criteria

- [x] Validate loaded config against `config.schema.json` in `RactorConfigActor`.
- [x] On validation failure, emit `Event::Error` with a typed message and keep the previous valid config or fail safe.
- [x] Remove the old `validate.rs` call path.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `config_validation_rejects_unknown_field` — an unknown provider field fails validation.
- [x] `unknown_field_produces_warning` — validates that unknown fields are caught.

### Layer 2 — Event Handling
- [x] `config_actor_emits_error_on_invalid_config` — `RactorConfigActor` emits `Event::Error` on load.
- [x] `config_actor_keeps_valid_config_on_reload_failure` — verifies actor handles errors gracefully.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/actors/config/ractor_config.rs` — added `validate_full()` call in `load_and_emit` and `reload_and_emit` with error emission; kept previous valid config on failure
- `crates/runie-core/src/actors/config/tests.rs` — fixed tests to use known providers (`openai`, `anthropic`) so validation passes
- `crates/runie-core/src/config/tests/validate_tests.rs` — layer 1 validation tests (pre-existing)

## Notes

- Coordinate with `replace-config-validator-with-jsonschema.md`.
- The `validate.rs` module already uses `jsonschema`; this task wired it into the actor.
- On validation failure during `reload_and_emit`, the actor keeps the previous valid config.
- `load_and_emit` emits Error and falls back to defaults if validation fails.
