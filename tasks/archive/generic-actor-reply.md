# Generic Actor Reply

**Status**: done
**Milestone": R4
**Category": Architecture / Actors
**Priority": P2

**Depends on": event-taxonomy-for-actor-state-sync
**Blocks": reduce-actor-handle-boilerplate

## Description

Add a generic reply type for actor request/response patterns. Currently each actor invents its own reply wrapper; this task standardizes it.

## Acceptance Criteria

- [ ] `Reply<T>` type in `actors/trait.rs`
- [ ] All actors use `Reply<T>` for request/response
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `reply_type_works`

### Layer 2 — Event Handling
- [ ] `actor_request_response_works`

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/actors/trait.rs`

## Notes

- `Reply<T>` already exists in `trait.rs`
- This task verifies it's used consistently
