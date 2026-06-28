# Type and unify provider/model layer

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: replace-config-validator-with-jsonschema
**Blocks**: extract-shared-streaming-response-parser

## Description

Provider and model configuration is largely stringly typed, with model names, tokenizers, context limits, and pricing parsed repeatedly in each provider. Introduce typed `Provider` and `Model` structs and a single model catalog that all providers share.

## Acceptance Criteria

- [ ] `Provider` config is represented by a typed enum/struct, not raw strings.
- [ ] A single model catalog contains ids, context limits, tokenizer assumptions, and pricing.
- [ ] Per-provider string parsing of model names is removed.
- [ ] Invalid provider/model combinations fail at config validation time.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_config_parses` — typed provider deserialization from config JSON.
- [ ] `model_catalog_lookup` — context limit and pricing resolve by model id.

### Layer 2 — Event Handling
- [ ] `config_actor_rejects_unknown_model` — ConfigActor emits an error for an unknown model id.

### Layer 3 — Rendering
- [ ] N/A — configuration has no TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `replay_switches_model` — a provider replay run uses the typed model catalog to pick context window behavior.

## Files touched

- `crates/runie-provider/src/config.rs`
- `crates/runie-provider/src/catalog.rs`
- `crates/runie-protocol/src/provider.rs`
- `crates/runie-core/src/config_validator.rs`
- `config.schema.json`

## Notes

- The catalog should be plain data (JSON/YAML) embedded with `include_str!` so adding a model does not require a code change.
- Avoid provider-specific tokenizer crates unless required for token counting.
