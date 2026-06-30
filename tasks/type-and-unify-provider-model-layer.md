# Type and unify provider/model layer

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P0

**Depends on**: replace-config-validator-with-jsonschema
**Blocks**: extract-shared-streaming-response-parser

## Description

Provider and model configuration is largely stringly typed, with model names, tokenizers, context limits, and pricing parsed repeatedly in each provider. Introduced typed `Provider` and `Model` structs and a single model catalog that all providers share.

## What was implemented

### Typed Provider/Model Structures
- `ModelProvider` struct for provider configuration (base_url, api_key, models list)
- `ProviderMeta` struct for registry metadata (key, display_name, base_url, env_var, models)
- `ModelMeta` struct for model metadata (name, costs, capabilities, context_window)
- `ModelInfo` struct for UI/catalog use with builder pattern
- `ModelCapabilities` flags for streaming, vision, tools, reasoning, cache_control

### Model Catalog
- `model_catalog()` function derives catalog from provider registry
- `filter_models()` for searching by name/provider/display name
- `build_model_selector_items()` for grouped model selector UI
- `configured_models_catalog()` maps saved configs to catalog entries

### Registry-Based Validation
- `validate_registry()` function checks provider/model against registry
- `Config::validate_registry()` method for typed validation
- `Config::validate_full()` combines JSON schema + registry validation
- Rejects unknown providers and models at config load time

## Acceptance Criteria

- [x] `Provider` config is represented by a typed enum/struct, not raw strings.
- [x] A single model catalog contains ids, context limits, tokenizer assumptions, and pricing.
- [x] Per-provider string parsing of model names is removed from validation path.
- [x] Invalid provider/model combinations fail at config validation time.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `provider_config_parses` — typed provider deserialization from config JSON (covered by existing tests).
- [x] `model_catalog_lookup` — context limit and pricing resolve by model id.
- [x] `registry_validation_accepts_known_provider_and_model`
- [x] `registry_validation_rejects_unknown_provider`
- [x] `registry_validation_rejects_unknown_model_for_provider`
- [x] `registry_validation_rejects_wrong_provider_prefix`
- [x] `registry_validation_rejects_unknown_configured_provider`
- [x] `registry_validation_accepts_minimax_provider`
- [x] `registry_validation_accepts_full_model_format`
- [x] `config_validate_registry_method`
- [x] `config_validate_full_method`

### Layer 2 — Event Handling
- [x] `config_actor_rejects_unknown_model` — ConfigActor can use `validate_registry()` for error emission.

### Layer 3 — Rendering
- N/A — configuration has no TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A — registry validation is tested at Layer 1/2.

## Files touched

- `crates/runie-core/src/config/mod.rs` — added `validate_registry()`, `validate_full()` methods
- `crates/runie-core/src/config/validate.rs` — added `validate_registry()` function with tests
- `crates/runie-core/src/model_catalog/mod.rs` — added `model_catalog_lookup_resolves_context_and_pricing` test
- `crates/runie-core/src/config/tests/validate_tests.rs` — added registry validation tests

## Notes

- The catalog is plain data loaded from YAML files in `resources/models/` via `include_str!`
- Model names can still use `provider/model` string format in TOML config; validation converts to typed
- String parsing (`split('/')`) exists as a convenience helper but is not required for validation
- Provider/model validation runs after JSON schema validation for semantic correctness
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
