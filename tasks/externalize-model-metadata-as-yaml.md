# Externalize provider/model metadata as YAML

**Status**: todo
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

- YAML model/provider metadata files live under `crates/runie-core/resources/models/`.
- `provider/registry_data.rs` and `model_catalog/*` static arrays are removed.
- A build script validates embedded metadata and generates a checksum manifest.
- User-provided metadata overrides are supported through `~/.runie/models/`.
- `cargo check --workspace` is green.

## Tests

- **Layer 1**: Parse and validate metadata files.
- **Layer 2**: Config loading with a custom user-provided model.
