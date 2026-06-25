# Unify event vocabularies

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Provider streams emit `ProviderEvent`s, which are translated to `Event`s, some persisted as `DurableCoreEvent`s, and some become `proto::EventMsg`s. Each translation is a source of ordering bugs, lost fields, stale indices, and duplicate `TurnComplete` events.

## Acceptance Criteria

- [x] One canonical `Event` enum exists in the core.
- [x] Provider stream (`ProviderEvent`), durable persistence (`DurableCoreEvent`), and protocol views (`EventMsg`) are separate types with `From` conversions.
- [x] `LLMEvent` renamed to `ProviderEvent` (already done).
- [x] `DurableCoreEvent` and `proto::EventMsg` are views with different purposes (not parallel vocabularies).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `provider_event_maps_to_event` — every `ProviderEvent` variant maps to `Event` without loss.
- [x] `text_and_tool_lifecycle_preserves_id` — id fields are preserved in lifecycle events.
- [x] `tool_and_message_events_become_durable` — tool calls and messages convert to durable events.
- [x] `transient_events_skip_durable` — streaming deltas do NOT become durable.

### Layer 2 — Event Handling
- [x] `llm_stream_maps_to_single_event_sequence` — synthetic `ProviderEvent` stream maps to canonical events.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] Existing replay fixtures cover multi-tool turn event ordering.

## Files touched

- `crates/runie-core/src/event/variants.rs` — flat `Event` enum
- `crates/runie-core/src/event/aliases.rs` — type aliases for backward compat
- `crates/runie-core/src/provider_event.rs` — canonical provider vocabulary
- `crates/runie-core/src/event/from_provider_event.rs` — `From<ProviderEvent> for Event`
- `crates/runie-core/src/event/to_durable.rs` — `Event::to_durable()` conversion
- `crates/runie-core/src/event/durable.rs` — `DurableCoreEvent` for persistence
- `crates/runie-protocol/src/event.rs` — `EventMsg` for IPC (separate vocabulary)

## Notes

The `proto::EventMsg` vocabulary is intentionally separate from `Event` because it's designed for IPC between processes with different serialization requirements. The `Event` type is the canonical internal vocabulary; `ProviderEvent` is the canonical external (provider) vocabulary; `DurableCoreEvent` is the persistence vocabulary. Each has distinct semantics and use cases.
