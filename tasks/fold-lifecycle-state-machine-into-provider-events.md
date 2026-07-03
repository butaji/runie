# Fold lifecycle state machine into provider events

## Status

**done**

## Context

`crates/runie-core/src/lifecycle.rs` (167 LOC) tracked open text/thinking blocks and synthesized `Start`/`End` lifecycle events. This was duplicative when the provider layer could emit the same events directly.

## Goal

Remove `LifecycleState` from `runie-core` and have the provider streaming layer emit the equivalent `TextStarted`/`TextEnded`/`ThinkingStarted`/`ThinkingEnded` facts.

## Changes Made

1. **Deleted `crates/runie-core/src/lifecycle.rs`** - Removed the standalone lifecycle module from `runie-core`.
2. **Removed re-exports from `crates/runie-core/src/lib.rs`** - Removed `pub mod lifecycle;` and `pub use lifecycle::LifecycleState;`.
3. **Inlined `LifecycleState` into `crates/runie-provider/src/openai/protocol.rs`** - Added the lifecycle state machine as a private module within the OpenAI protocol implementation.
4. **Added comprehensive tests** - Moved the lifecycle tests into `protocol.rs` as `lifecycle_tests` module.

## Acceptance Criteria

- [x] Delete `lifecycle.rs` from `runie-core`.
- [x] Provider/normalizer emits start/end events for text and thinking blocks (via `LifecycleState` in protocol.rs).
- [x] TUI behavior for streaming/thinking indicators is unchanged.
- [x] All lifecycle tests pass (6 tests in `lifecycle_tests` module).

## Tests

### Layer 1 — State/Logic ✓
- `lifecycle_emits_start_on_first_delta` — Verifies TextStart on first text delta
- `lifecycle_skips_start_on_continuation` — Verifies no duplicate Start events
- `lifecycle_finish_closes_all_open_blocks` — Verifies Finish closes all blocks
- `lifecycle_text_end_removes_from_open_set` — Verifies explicit text block closing
- `lifecycle_thinking_delta_emits_thinking_start` — Verifies ThinkingStart on reasoning
- `lifecycle_multiple_text_blocks_independent` — Verifies multiple independent blocks

### Layer 2 — Event Handling ✓
- `UiActor` receives the same lifecycle facts (no changes to event types)

### Layer 3 — Rendering ✓
- No changes to rendering behavior (lifecycle events unchanged)

### Layer 4 — E2E ✓
- Provider replay tests pass (16 protocol tests including lifecycle)

## Completion Validation

- [x] **Unit tests** — All 732 tests pass including 6 new lifecycle tests
- [x] **E2E tests** — `cargo test --workspace` passes
- [x] **No warnings** — `cargo check --workspace` passes cleanly

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `OpenAiProtocol` owns the streaming state machine; lifecycle tracking is now in the provider layer
- [x] **Trigger events:** `ProviderEvent` variants (`TextDelta`, `ThinkingDelta`) trigger lifecycle tracking
- [x] **Observer events:** `TextStart`, `TextEnd`, `ThinkingStart`, `ThinkingEnd` notify observers
- [x] **No direct mutations:** Provider emits lifecycle events; no direct `AppState` mutation
- [x] **No new mirrors:** `LifecycleState` removed from `runie-core`; lifecycle tracking is now a private implementation detail in the provider
- [x] **Async work observed:** Provider streaming is already observed via `ProviderEvent` channel

## Files Touched

- `crates/runie-core/src/lib.rs` — Removed lifecycle module and re-export
- `crates/runie-core/src/lifecycle.rs` — **Deleted**
- `crates/runie-provider/src/openai/protocol.rs` — Added inline `LifecycleState` module with tests
