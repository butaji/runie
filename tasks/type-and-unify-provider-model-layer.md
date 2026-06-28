# Type and unify the provider / model layer

**Status**: todo
**Milestone**: R2
**Category**: Provider / Configuration
**Priority**: P1

**Depends on**: route-cli-config-through-configactor, unify-provider-credential-resolution-with-dotenvy
**Blocks**: none

## Description

The provider/model layer has three parallel representations that duplicate fields and require manual copying:
- `crates/runie-provider/src/openai/protocol.rs` and `stream.rs` navigate `serde_json::Value` by hand.
- `crates/runie-provider/src/openai/request.rs` mixes typed message structs with untyped `json!` request bodies.
- `crates/runie-core/src/provider/registry_data.rs` defines `ProviderYaml`/`ModelYaml` that are copied field-by-field into `ProviderMeta`/`ModelMeta`.
- `crates/runie-core/src/model_catalog/mod.rs` defines `ModelInfo`/`ModelCapabilities` that mirror `ModelMeta`.
- `crates/runie-core/src/provider/config.rs`, `model/state/domain_ops.rs`, and `runie-provider/src/factory.rs` each build their own configured-provider tuples.

The Pareto fix is to type the OpenAI SSE/request structs with `serde` derives, collapse the YAML + runtime + catalog metadata into one type family, and provide a single `ConfiguredProvider` view.

## Acceptance Criteria

- [ ] Define typed `Deserialize` structs for OpenAI SSE chunks (`Chunk`, `Delta`, `Choice`, `Usage`, tool-call deltas) in `runie-provider/src/openai/protocol.rs`.
- [ ] Define a typed `ChatCompletionsRequest` struct in `runie-provider/src/openai/request.rs` and build the body with `serde_json::to_value` once.
- [ ] Add `#[derive(Deserialize)]` with `#[serde(default)]` directly on `ProviderMeta`/`ModelMeta` and delete `ProviderYaml`/`ModelYaml` intermediates.
- [ ] Implement `From<&ModelMeta>` for `ModelInfo`/`ModelCapabilities` (or collapse them) and generate the selector list from the registry.
- [ ] Add `Config::configured_providers()` / `Config::default_model()` and make `provider/config.rs`, `model/state/domain_ops.rs`, and `runie-provider/src/factory.rs` use it.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `openai_chunk_deserializes` — representative SSE chunk parses into typed structs.
- [ ] `request_serializes_to_expected_shape` — `ChatCompletionsRequest` serializes to the same JSON the old `json!` builder produced.
- [ ] `configured_providers_unified` — all three call sites produce the same provider/model tuples.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_replay_after_typing` — MiniMax replay fixtures still parse and produce identical provider events.

## Files touched

- `crates/runie-provider/src/openai/protocol.rs`
- `crates/runie-provider/src/openai/stream.rs`
- `crates/runie-provider/src/openai/request.rs`
- `crates/runie-core/src/provider/registry_data.rs`
- `crates/runie-core/src/provider/registry.rs`
- `crates/runie-core/src/model_catalog/mod.rs`
- `crates/runie-core/src/provider/config.rs`
- `crates/runie-core/src/model/state/domain_ops.rs`
- `crates/runie-provider/src/factory.rs`

## Notes

- `goose` and `jcode` both deserialize provider SSE payloads into typed structs rather than navigating `serde_json::Value`.
- This task should land after config/credential routing is stable so the `ConfiguredProvider` view can read from `RactorConfigActor`.
- Keep the `ProviderProtocol` trait intact; only clean up the duplicate helper/tool-accumulator code.
