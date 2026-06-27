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

### Phase 1: Foundation (COMPLETE)
- [x] `rig-core` added to workspace dependencies
- [x] `rig_adapter` module created with event mapping infrastructure
- [x] `RigOpenAiProvider` adapter struct created
- [x] Event mapping functions: `map_streamed_content`, `chat_message_to_rig`
- [x] All adapter unit tests pass (12 tests)
- [x] `cargo check --workspace` is green

### Phase 2: Full Migration (PENDING)
- [ ] Complete `RigOpenAiProvider::generate()` with rig-core streaming
- [ ] Complete `RigOpenAiProvider::generate_with_tools()` with rig-core tool support
- [ ] Replace custom SSE parsing with rig-core streaming
- [ ] Replace custom protocol state machines with rig-core adapters
- [ ] Verify all existing providers work via rig-core adapters
- [ ] Preserve mock/replay fixture testing

## Acceptance Criteria

- [x] `rig-core` is added to workspace dependencies.
- [x] `crates/runie-provider/src/rig_adapter.rs` provides foundation for rig-core integration.
- [ ] Custom SSE/protocol implementations removed or reduced to thin adapters.
- [ ] All existing providers (OpenAI, OpenRouter, DeepSeek, Groq, Together, etc.) continue to work via `rig-core` adapters.
- [ ] Replay/mock fixture testing is preserved with a thin adapter layer.
- [x] `cargo check --workspace` is green with no new warnings.

## Files Modified

- `Cargo.toml` - added rig-core workspace dependency
- `crates/runie-provider/Cargo.toml` - added rig-core and pin-project
- `crates/runie-provider/src/rig_adapter.rs` - adapter module with event mapping

## Tests

- **Layer 1**: Pure adapter tests for request normalization and event mapping. ✓ (12 tests passing)
- **Layer 4**: Provider-replay tests with captured SSE fixtures verify event ordering and tool-call deltas. ✓ (existing tests pass)

## Notes

### Phase 1 Foundation
The adapter module establishes the foundation for rig-core integration:
1. Event mapping functions correctly translate rig-core streaming events to Runie's ProviderEvent types
2. The adapter struct is in place and implements the `Provider` trait
3. Current `generate()` method returns an error indicating streaming integration is pending

### Phase 2 Migration Path
To complete the migration:
1. Fix `RigOpenAiProvider::generate()` to use rig-core's streaming client
2. Note: `rig_core::stream()` method is `pub(crate)` - requires using trait methods or alternative approach
3. Verify API compatibility with rig-core v0.39.0
4. Test with real provider credentials

### API Notes
- `rig-core` v0.39.0 has `CompletionClient` trait for building completion models
- The `stream()` method on completion models is `pub(crate)`, not public
- May need to use trait methods or find public API for streaming
