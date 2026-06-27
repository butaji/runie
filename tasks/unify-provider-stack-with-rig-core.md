# Unify provider stack with `rig-core`

**Status**: in_progress
**Milestone**: R4
**Category**: Provider
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Summary

Replace the custom `runie-provider` OpenAI/SSE/protocol/registry implementation with `rig-core` provider abstractions. Keep session storage in JSON(L); do not introduce SQLite. Map `rig-core` streaming and tool events into Runie's event bus.

## Implementation Status

### Phase 1: Foundation (COMPLETED)
- [x] `rig-core` added to workspace dependencies
- [x] `rig_adapter` module created with event mapping infrastructure
- [x] `RigOpenAiProvider` adapter struct created (placeholder implementation)
- [x] Event mapping functions: `map_finish_reason`, `map_streaming_choice`
- [x] All existing tests pass
- [x] `cargo check --workspace` is green

### Phase 2: Full Migration (TODO)
- [ ] Complete `RigOpenAiProvider::generate()` with rig-core streaming
- [ ] Complete `RigOpenAiProvider::generate_with_tools()` with rig-core tool support
- [ ] Replace custom SSE parsing with rig-core streaming
- [ ] Replace custom protocol state machines with rig-core adapters
- [ ] Verify all existing providers work via rig-core adapters
- [ ] Preserve mock/replay fixture testing

## Acceptance Criteria

- `rig-core` is added to workspace dependencies. ✓
- `crates/runie-provider/src/openai/*`, `framing.rs`, `retry.rs`, `protocol.rs`, and provider-registry boilerplate are removed or reduced to thin adapters. (partial - adapter module created, migration pending)
- All existing providers (OpenAI, OpenRouter, DeepSeek, Groq, Together, etc.) continue to work via `rig-core` adapters. (pending)
- Replay/mock fixture testing is preserved with a thin adapter layer. (pending)
- `cargo check --workspace` is green with no new warnings. ✓

## Files Modified

- `Cargo.toml` - added rig-core workspace dependency
- `crates/runie-provider/Cargo.toml` - added rig-core and pin-project
- `crates/runie-provider/src/rig_adapter.rs` - new adapter module (13KB)

## Tests

- **Layer 1**: Pure adapter tests for request normalization and event mapping. ✓
- **Layer 4**: Provider-replay tests with captured SSE fixtures verify event ordering and tool-call deltas. ✓ (existing tests pass)

## Notes

The adapter module establishes the foundation for rig-core integration:
1. Event mapping functions correctly translate rig-core streaming events to Runie's ProviderEvent types
2. The adapter struct is in place but streaming implementation is pending
3. All existing functionality is preserved; migration can proceed incrementally

The placeholder implementation returns errors indicating that rig-core streaming is not yet implemented. Full migration requires completing the async streaming integration with proper rig-core client setup and request/response handling.
