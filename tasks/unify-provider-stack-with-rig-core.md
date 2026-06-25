# Unify provider stack with `rig-core`

**Status**: todo
**Milestone**: R4
**Category**: Provider
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Summary

Replace the custom `runie-provider` OpenAI/SSE/protocol/registry implementation with `rig-core` provider abstractions. Keep session storage in JSON(L); do not introduce SQLite. Map `rig-core` streaming and tool events into Runie's event bus.

## Acceptance Criteria

- `rig-core` is added to workspace dependencies.
- `crates/runie-provider/src/openai/*`, `framing.rs`, `retry.rs`, `protocol.rs`, and provider-registry boilerplate are removed or reduced to thin adapters.
- All existing providers (OpenAI, OpenRouter, DeepSeek, Groq, Together, etc.) continue to work via `rig-core` adapters.
- Replay/mock fixture testing is preserved with a thin adapter layer.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Pure adapter tests for request normalization and event mapping.
- **Layer 4**: Provider-replay tests with captured SSE fixtures verify event ordering and tool-call deltas.
