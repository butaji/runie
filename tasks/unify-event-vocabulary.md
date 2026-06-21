# Unify event vocabularies

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Provider streams emit `LLMEvent`s, which are translated to `AgentEvent`s, some persisted as `DurableCoreEvent`s, and some become `proto::EventMsg`s. `runie-tui` then defines `EffectCommand` for side effects. Each translation is a source of ordering bugs, lost fields, stale indices, and duplicate `TurnComplete` events.

## Acceptance Criteria

- [ ] One canonical `Event` enum exists in the core.
- [ ] Provider stream, durable persistence, and protocol views are derived via `From` traits or thin adapters.
- [ ] `LLMEvent` is removed if `AgentEvent` already covers the same vocabulary.
- [ ] `DurableCoreEvent` and `proto::EventMsg` are views, not parallel vocabularies.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_vocabulary_conversion_round_trips` — every variant maps to provider/durable/proto views without loss.

### Layer 2 — Event Handling
- [ ] `llm_stream_maps_to_single_event_sequence` — feed a synthetic `LLMEvent` stream and assert one canonical event per logical action.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_m3_multi_tool_turn_event_order` — replay fixture and assert `TurnComplete` appears exactly once and last.

## Files touched

- `crates/runie-core/src/event/variants/mod.rs`
- `crates/runie-core/src/llm_event.rs`
- `crates/runie-core/src/proto/event.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-tui/src/effects/mod.rs`
- Provider parsing code that emits `LLMEvent`.

## Notes

This is the highest long-term payoff for async/event test stability. Keep the canonical enum small; views can add provider-specific metadata.
