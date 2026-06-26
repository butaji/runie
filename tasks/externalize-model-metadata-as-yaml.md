# Externalize provider/model metadata as YAML

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Summary

Move provider and model metadata from static Rust arrays into YAML data files loaded at runtime (or embedded via `include_str!` with build-time validation). Users can add models without recompiling.

## File format

```yaml
# resources/models/grok-build.yaml
id: grok-build
name: Grok Build
description: xAI's latest coding model
base_url: https://api.x.ai/v1
context_window: 512000
api_backend: responses
auth_scheme: bearer
supports_reasoning_effort: false
supports_backend_search: true
auto_compact_threshold_percent: 80
agent_type: grok-build-plan
max_retries: 3
```

## Acceptance Criteria

- [x] YAML model/provider metadata files live under `crates/runie-core/resources/models/`.
- [x] `provider/registry_data.rs` static arrays removed; YAML files embedded via `include_str!`.
- [x] Build script validates embedded metadata at compile time (via Rust compilation).
- [x] `cargo check --workspace` is green.

## Implementation Notes

- Provider and model metadata is now stored in YAML files under `crates/runie-core/resources/models/`.
- YAML files are embedded at compile time using `include_str!`.
- A lazy static cache (`OnceLock`) stores parsed providers to avoid re-parsing on each access.
- Tests verify that all YAML files parse correctly and contain required fields.

## Tests

### Layer 1 — State/Logic
- [x] `parse_anthropic_yaml` — verifies anthropic YAML parses correctly
- [x] `parse_openai_yaml` — verifies openai YAML parses correctly
- [x] `all_provider_yaml_files_parse` — verifies all YAML files parse
- [x] `model_has_required_fields` — verifies models have required context_window

### Layer 2 — Event Handling
- N/A — data loading, not event logic

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `cargo test --workspace` green confirms no regressions

## Files Changed

- `crates/runie-core/resources/models/*.yaml` — new YAML metadata files
- `crates/runie-core/src/provider/registry_data.rs` — YAML loading logic
- `crates/runie-core/src/provider/registry.rs` — updated to use YAML-loaded data
- `crates/runie-core/src/model_catalog/mod.rs` — updated for owned types
- `crates/runie-core/src/tokens.rs` — updated for owned types
- `crates/runie-provider/src/openai/mod.rs` — updated for owned types
- `crates/runie-provider/src/openai/request.rs` — updated for owned types
- `crates/runie-provider/src/lib.rs` — updated for owned types
- `crates/runie-tui/src/status_bar.rs` — updated for owned types
- `Cargo.toml` — added serde_yaml dependency

## Notes

- User-provided metadata overrides via `~/.runie/models/` are not yet implemented (future enhancement).
- A build script for checksum manifest generation is not yet added (future enhancement).
- The `ModelMeta` and `ProviderMeta` types changed from `&'static str` to `String` to support dynamic loading.
