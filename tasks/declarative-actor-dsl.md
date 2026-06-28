# Declarative Actor DSL

**Status**: done
**Milestone**: R5
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks**: unified-dsl-intents-for-state-mutations

## Description

Define a declarative DSL for defining actors and their message handlers. The goal is to reduce boilerplate in actor implementations.

## Implementation

Added `define_actor!` macro in `crates/runie-macros/src/actor.rs`.

The macro generates:
- Actor struct with state mutex and bus bridge
- Ractor trait implementation (`pre_start`, `handle`)
- `spawn` function for actor creation
- `apply_to` method on message enum for synchronous testing

### Usage

```rust
define_actor! {
    name: MyActor,
    msg: MyMsg,
    state: MyState,
    events: MyEvent,

    impl handle(msg, state, bus) {
        MyMsg::Increment => {
            state.value += 1;
            bus.publish(MyEvent::Changed);
        }
        MyMsg::SetValue { value } => {
            state.value = *value;
            bus.publish(MyEvent::Changed);
        }
    }
}
```

## Acceptance Criteria

- [x] `define_actor!` macro defined
- [x] Message handlers use declarative syntax
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `define_actor_generates_valid_code` - Macro generates compilable Rust code
- [x] `message_patterns_match` - Message enum patterns work correctly
- [x] `state_default_is_zero` - State types work correctly
- [x] `event_carries_value` - Event types work correctly

### Layer 2 — Event Handling
- N/A (macro generates code; runtime behavior tested via actor tests)

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A

## Files touched

- `crates/runie-macros/src/actor.rs` (new)
- `crates/runie-macros/src/lib.rs` (added macro export)
- `crates/runie-macros/tests/actor_macro_test.rs` (new test file)

## Notes

- The macro uses ractor as the underlying actor framework
- Generated `apply_to` method allows synchronous testing without spawning actors
- The macro follows the existing actor patterns in the codebase
