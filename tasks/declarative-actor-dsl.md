# Declarative actor composition DSL

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync
**Blocks**: config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, unified-dsl-intents-for-state-mutations

## Goal

Provide a small, composable DSL that hides actor-message boilerplate and makes state interactions declarative at every layer. Instead of manually constructing `Msg` enums and sending them through `tx` handles, code reads like a data-flow description:

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

- [x] `crates/runie-core/src/dsl/` exists with `flow.rs`, `intent.rs`, `fact.rs`, and `effect.rs`.
- [x] `Intent` enum re-exported from `event/intent.rs` via `dsl/intent.rs`.
- [x] `Fact` enum in `dsl/fact.rs`.
- [x] `Effect` type in `dsl/effect.rs`.
- [x] `Flow` and combinators in `dsl/flow.rs`.
- [x] `Runtime` trait, `TestRuntime`, `RealRuntime` in `dsl/runtime.rs`.
- [x] At least the following flows are demonstrated using the DSL:
  - `/theme <name>` command (via Intent::SetTheme).
  - `ToggleVimMode` control event (via Intent::ToggleVimMode + Fact::ViewInvalidated).
  - `SessionSaved` → notification.
  - `Enter` in input (via Intent::Submit composition).
- [x] The DSL supports unit-testing flows without a runtime: `TestRuntime` records sent intents/facts/effects.
- [x] No macro magic required for simple flows; advanced combinators may use macros only if they reduce duplication.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `dsl_flow_records_intent_in_test_runtime` — a constructed flow sends the expected intent to a test runtime.
- [x] `dsl_branch_combinator_is_callable` — `.branch()` can be called with valid inputs.
- [x] `dsl_flow_composes_with_then` — `.then()` chains flows correctly.
- [x] `dsl_flow_records_multiple_steps` — flows with multiple steps record all steps.
- [x] `dsl_notify_records_content_and_level` — `.notify_level()` records both content and level.
- [x] `dsl_fact_broadcasts_to_runtime` — `.fact()` broadcasts to runtime.
- [x] `effect_runs_in_test_runtime` — effects execute in TestRuntime.
- [x] `theme_command_flow_emits_set_theme_intent` — theme flow produces `Intent::SetTheme`.
- [x] `toggle_vim_mode_flow_emits_intent_and_fact` — ToggleVimMode flow produces both intent and fact.
- [x] `session_saved_flow_emits_notification` — SessionSaved flow produces notification.
- [x] `submit_input_flow_composes_with_then` — submit flow composition works.

### Layer 2 — Event Handling
- [x] All DSL tests pass with TestRuntime.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/dsl/mod.rs` — DSL entry point and exports.
- `crates/runie-core/src/dsl/flow.rs` — `Flow`, `Step`, `on()`, and all combinators.
- `crates/runie-core/src/dsl/runtime.rs` — `Runtime` trait, `TestRuntime`, `RealRuntime`, and thread-local helpers.
- `crates/runie-core/src/dsl/intent.rs` — `Intent` re-export from `event/intent.rs`.
- `crates/runie-core/src/dsl/fact.rs` — `Fact` enum for broadcast state changes.
- `crates/runie-core/src/dsl/effect.rs` — `Effect` type for fire-and-forget IO requests.
- `crates/runie-core/src/dsl/examples.rs` — DSL usage examples demonstrating the pattern.

## Notes

- The DSL is optional for internal actor logic but **mandatory** for command/dialog/input handlers.
- Keep it tiny. If a combinator does not have three real use cases, do not add it.
- Rejected alternative: heavy macro-based effect system. Rejected because macros hurt readability and compile times.
- Full integration of the DSL into command handlers requires the actor ownership migration tasks to be completed first.
- Current state: DSL infrastructure is complete with examples. Full adoption pending actor migrations.
