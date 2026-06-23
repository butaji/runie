# Declarative actor composition DSL

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations

## Goal

Provide a small, composable DSL that hides actor-message boilerplate and makes state interactions declarative at every layer. Instead of manually constructing `Msg` enums and sending them through `tx` handles, code should read like a data-flow description:

```rust
// Command handler
on(Command::Theme(name))
    .intent(ConfigIntent::SetTheme(name))
    .notify("Theme updated")

// Input submit
on(InputEvent::Submit)
    .map(input_actor::take_submit_content)
    .branch(
        is_empty => dismiss(),
        has_content => intent(SessionIntent::AddUserMessage)
                       .then(TurnIntent::RunIfQueued)
    )

// Turn progress
on(TurnFact::ResponseDelta { delta })
    .intent(SessionIntent::AppendAssistantText(delta))
    .view(ViewIntent::Invalidate)
```

The DSL is **not** a new runtime; it compiles to the same actor messages and events. It is a thin, type-safe veneer that makes the actor-ownership model ergonomic and keeps business logic declarative.

## Primitives

| Primitive | Meaning | Example |
|-----------|---------|---------|
| `on(trigger)` | Start a flow from a trigger (event, command, key). | `on(Event::SubmitInput)` |
| `.intent(I)` | Send an intent to its owning actor. | `.intent(ConfigIntent::SetTheme(...))` |
| `.fact(F)` | Broadcast a fact (used inside actors). | `.fact(SessionFact::Changed)` |
| `.map(f)` | Pure transformation of the trigger value. | `.map(|e| e.content)` |
| `.filter(pred)` | Drop the flow unless predicate holds. | `.filter(|s| !s.is_empty())` |
| `.branch((pred, flow), ...)` | Conditional composition. | `.branch((is_empty, dismiss), (otherwise, proceed))` |
| `.then(flow)` | Sequence another flow. | `.intent(A).then(.intent(B))` |
| `.select(selector)` | Read immutable projection. | `.select(|state| state.config.theme_name)` |
| `.notify(text)` | Show a transient notification. | `.notify("Saved")` |
| `.effect(f)` | Perform a pure side-effect (clipboard, IO request). | `.effect(|_| io_actor::copy(...))` |
| `.none` | No-op terminal. | `.none` |

## Composition rules

- A flow is either an **intent** (ends with an actor message), a **fact** (ends with a broadcast), an **effect** (ends with an IO request), or a **notification**.
- Flows compose with `.then` only when the left-hand output type matches the right-hand input type or when the right-hand side ignores the input (`then_unit`).
- The DSL is implemented as a small set of builder structs in `crates/runie-core/src/dsl/`; it does not use macros unless absolutely necessary.
- Actor handles are captured at construction time so the DSL is zero-cost at runtime.

## Acceptance criteria

- [ ] `crates/runie-core/src/dsl/` exists with `flow.rs`, `intent.rs`, `fact.rs`, `effect.rs`, and a top-level `on` function.
- [ ] Every actor intent type (`ConfigIntent`, `SessionIntent`, `InputIntent`, `ViewIntent`, `TurnIntent`, `PermissionIntent`, `NotificationIntent`, `TrustIntent`, `EnvIntent`) implements `Into<Intent>` so it can be used uniformly.
- [ ] At least the following flows are rewritten using the DSL as reference examples:
  - `/theme <name>` command.
  - `Enter` in input.
  - `ToggleVimMode` control event.
  - `SessionSaved` → notification.
- [ ] The DSL supports unit-testing flows without a runtime: a `TestRuntime` records sent intents/facts/effects.
- [ ] No macro magic required for simple flows; advanced combinators may use macros only if they reduce duplication.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `dsl_flow_records_intent_in_test_runtime` — a constructed flow sends the expected intent to a test runtime.
- [ ] `dsl_branch_selects_correct_arm` — `.branch` chooses the arm matching the predicate.

### Layer 2 — Event Handling
- [ ] `theme_command_flow_emits_set_theme_intent` — `/theme` flow produces `ConfigIntent::SetTheme`.
- [ ] `submit_input_flow_emits_session_and_turn_intents` — Enter flow produces both `SessionIntent::AddUserMessage` and `TurnIntent::RunIfQueued`.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/dsl/mod.rs` — DSL entry point and `on` constructor.
- `crates/runie-core/src/dsl/flow.rs` — flow builder and combinators.
- `crates/runie-core/src/dsl/runtime.rs` — real runtime (sends actor messages) and `TestRuntime`.
- `crates/runie-core/src/dsl/intent.rs` — unified `Intent` enum + actor-specific intent traits.
- `crates/runie-core/src/event/` — ensure intent/fact types implement needed traits.
- `crates/runie-core/src/commands/dsl/handlers/system.rs` — rewrite `/theme` as a DSL example.
- `crates/runie-core/src/update/input/text.rs` — rewrite submit flow as DSL example.

## Notes

- The DSL is optional for internal actor logic but **mandatory** for command/dialog/input handlers.
- Keep it tiny. If a combinator does not have three real use cases, do not add it.
- Rejected alternative: heavy macro-based effect system. Rejected because macros hurt readability and compile times.
