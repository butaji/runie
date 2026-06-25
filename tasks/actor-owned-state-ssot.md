# Actor-owned state with SSOT and unified DSL

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: declarative-actor-dsl, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, actor-lifecycle-and-handle-registry, test-actor-harness, config-ssot-via-configactor, session-actor-owns-session-state, input-actor-owns-input-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results, ui-control-actor-owns-dialog-state, unified-dsl-intents-for-state-mutations, remove-direct-appstate-mutations

## Goal

Every piece of mutable application state is owned by exactly one actor. Handlers, commands, and dialogs never mutate `AppState` directly; they emit typed intents. The owning actor applies the intent, persists or side-effects as needed, and publishes a state-changed event. `AppState` becomes a read-only projection that is updated only through events.

This eliminates logic spreading, gives every subsystem a single source of truth, and lets the UI speak a small, uniform, composable DSL of intents across the whole app. Complexity lives inside actors; the code that wires them together is declarative.

## Architecture

```text
Commands / Dialogs / Input handlers
            │
            ▼
    ┌───────────────┐
    │  IntentEvent  │   ("what the user wants")
    └───────────────┘
            │
    ┌───────┴───────┐
    ▼               ▼
 ConfigActor    SessionActor    TurnActor    InputActor    ViewActor    ...
    │               │               │            │            │
    └───────────────┴───────────────┴────────────┴────────────┘
            │
            ▼
    StateChangedEvent  ("what changed")
            │
            ▼
       AppState projection
            │
            ▼
       RenderActor / UiActor
```

### Ownership map

| State slice | Owner | Current violators | Key events / intents |
|-------------|-------|-------------------|----------------------|
| `ConfigState` + `config_cache` | `ConfigActor` | slash handlers, settings dialog, login flow, `provider_model_toggle` | `ConfigMsg::*` → `Event::ConfigLoaded` |
| `SessionState` (messages, tree, edits, attachments, metadata) | `SessionActor` | `update/input/text.rs`, `update/agent/core.rs`, `update/session.rs`, `commands/dsl/handlers/session/*`, `session_replay.rs` | `SessionMsg::*` → `Event::SessionChanged` |
| `InputState` | `InputActor` | `update/input/*`, `update/system.rs`, `update/session.rs`, `commands/dsl/handlers/*` | `InputMsg::*` → `Event::InputChanged` |
| `ViewState` + derived caches | `ViewActor` | every `mark_dirty()` / `messages_changed()` caller, `model/cache.rs`, dialog openers | `ViewMsg::*` → `Event::ViewChanged` |
| `CompletionState` + ghost/tab | `CompletionActor` | `update/path_complete.rs`, `update/dialog/tab_complete.rs`, `update/agent/at_refs.rs` | `CompletionMsg::*` → `Event::CompletionChanged` |
| `AgentState` turn lifecycle + queues | `TurnActor` | `update/agent/core.rs`, `update/session.rs`, `update/system.rs`, `runie-agent/src/actor.rs` handle | `TurnMsg::*` → `Event::TurnProgress` |
| `approval_registry` + `permission_request` | `PermissionActor` | `update/permission.rs`, `update/input/mod.rs`, `runie-agent/src/emit_approval_sink.rs` | `PermissionMsg::*` → `Event::PermissionRequest` / `PermissionResolved` |
| `trust_decisions` + read-only flag | `TrustActor` (thin; persists via `PersistenceActor`) | `update/system.rs`, `commands/dsl/handlers/tool.rs` | `TrustMsg::*` → `Event::TrustChanged` |
| `transient_message*` | `NotificationActor` | `update/system.rs`, `update/dispatch.rs`, `update/dialog/router.rs`, `notification.rs`, `model/cache.rs` | `NotificationMsg::*` → `Event::TransientMessage` / `ClearTransient` |
| `fff_file_results` + `fff_debounce` | `FffIndexerActor` | `update/dialog/open.rs`, `update/dialog/file_picker.rs` | already emits `Event::FffSearchResult`; consume it |
| `git_info` + `cwd_name` | `EnvActor` (or reuse `IoActor`) | `runie-tui/src/app_init.rs` | `EnvMsg::Detect` → `Event::EnvDetected` |
| `open_dialog`, `dialog_back_stack`, `login_flow`, `should_quit` | `UiControlActor` | `update/dialog/*.rs`, `login_flow/`, `commands/dsl/handlers/system.rs` | `UiControlMsg::*` → `DialogOpened` / `QuitRequested` |

### Event taxonomy

Two event families:

1. **Intents** (fire-and-forget requests to an actor). Naming convention: verb-noun, e.g. `SubmitInput`, `SwitchModel`, `ShowNotification`, `AskPermission`.
2. **Facts** (broadcast state changes). Naming convention: noun-past-tense, e.g. `ConfigLoaded`, `SessionChanged`, `InputChanged`, `TurnProgress`, `PermissionResolved`.

No handler consumes an intent and mutates state in the same breath. The only place a fact is produced is inside the owning actor.

### AppState contract

- `AppState` fields `session`, `input`, `view`, `completion`, `agent`, `config`, `trust_decisions`, `transient_*`, `fff_*`, `git_info`, `cwd_name`, `permission_request`, `open_dialog`, `dialog_back_stack`, `login_flow`, `should_quit` become private to writes.
- Inner state structs (`SessionState`, `InputState`, `ViewState`, `CompletionState`, `AgentState`, `ConfigState`) are also encapsulated.
- Provide immutable accessors: `state.session()`, `state.input()`, `state.view()`, etc.
- `AppState::update(event: Fact)` dispatches facts to projection helpers; intents are illegal here and should be statically typed away (or panic in debug builds).
- `AppState` stores one `ActorHandles` registry instead of loose `Option<Sender>` fields.

### DSL uniformity

All slash commands, palette items, and dialog actions produce the same kind of thing: an `Event::Intent(_)` or a concrete intent variant. There is no special-case direct mutation for "simple" commands. Examples:

- `/theme runie` → `Event::SetTheme { name: "runie" }` → `ConfigActor` → `Event::ConfigLoaded`.
- `/trust` → `Event::SetTrust { path, decision: Trusted }` → `TrustActor` → `Event::TrustChanged`.
- `Enter` in input → `Event::SubmitInput` → `InputActor` → maybe `Event::UserMessageAppended` (SessionActor) + `Event::RunTurn` (TurnActor).

## Execution phases

1. **Foundation**: define `Intent`/`Fact` split (`event-taxonomy-for-actor-state-sync`), introduce `ActorHandles` registry (`actor-lifecycle-and-handle-registry`), add `TestActorHarness` (`test-actor-harness`).
2. **Projection**: make `AppState` a read-only projection with private fields (`app-state-read-only-projection`). This is staged: lint first, fix incrementally, then `trybuild`.
3. **Declarative DSL**: build the `on(...).intent(...)` DSL (`declarative-actor-dsl`).
4. **Config SSOT**: finish `ConfigActor` ownership (`config-ssot-via-configactor`).
5. **Session actor**: move all message/session-tree/pending-edit mutations into `SessionActor` (consider renaming existing durability logger to `SessionLogActor`).
6. **Turn actor**: extract turn lifecycle and queues from `update/agent/core.rs` into `TurnActor`.
7. **Input/View/Completion actors**: move input editing, view caches, and completion popups into their actors.
8. **Cross-cutting actors**: `PermissionActor`, `NotificationActor`, `TrustActor`, `EnvActor`, `FffIndexerActor`, `UiControlActor`.
9. **DSL cleanup**: rewrite all commands/dialogs using the declarative DSL (`unified-dsl-intents-for-state-mutations`).
10. **Final sweep**: delete `mark_dirty()` / `messages_changed()` helpers, enforce the no-direct-mutation gate (`remove-direct-appstate-mutations`), run full test suite.

## Acceptance criteria

- [x] Every production write to an `AppState` field is traceable to a single actor. (tracked in child tasks)
- [x] No command, dialog handler, or input handler mutates `AppState` directly. (tracked in child tasks)
- [x] `AppState` exposes only immutable accessors to UI/render code. (tracked in child tasks)
- [x] A `grep` for direct field assignments (`state\.[a-z_]+\s*=`) outside `AppState` impl and actors returns zero hits. (tracked in child tasks)
- [x] All state changes reach the UI through events. (tracked in child tasks)
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `app_state_projection_rejects_direct_mutation` — compile-time or debug-only test that writing to a private field fails.
- [ ] `actor_ownership_table_is_exhaustive` — every `AppState` field is mapped to an actor in a registry test.

### Layer 2 — Event Handling
- [ ] `intent_event_routes_to_config_actor` — `SetTheme` intent results in `ConfigLoaded` fact.
- [ ] `intent_event_routes_to_session_actor` — `AddUserMessage` intent results in `SessionChanged` fact.

### Layer 3 — Rendering
- [ ] `view_actor_invalidate_triggers_render` — `ViewMsg::Invalidate` causes `RenderActor` to draw.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `mock_provider_turn_uses_only_actor_intents` — run a full turn and assert no direct `AppState` mutation events occur.

## Files touched

- `crates/runie-core/src/model/state/app_state.rs` — make fields private, add accessors, add actor-handle registry.
- `crates/runie-core/src/event/` — add intent/fact variants.
- `crates/runie-core/src/actors/` — new actor modules.
- `crates/runie-core/src/update/` — handlers become intent producers.
- `crates/runie-core/src/commands/` — commands emit intents.
- `crates/runie-core/src/model/cache.rs` — view cache refresh driven by `ViewActor`.
- `crates/runie-core/src/notification.rs` — delegate to `NotificationActor`.
- `crates/runie-tui/src/app_init.rs` — env detection via `EnvActor`.

## Notes

- This task is the parent spec. Do not implement it directly; implement the child tasks it blocks.
- Existing tasks `dedupe-config-actor-mutations`, `consolidate-settings-providers-dialog`, `remove-login-config-test-shim`, `unify-provider-modules`, and `move-provider-catalog-to-provider-crate` are folded into this program; update their descriptions to reference this task and the `config-ssot-via-configactor` child.
- Rejected alternative: keep `UiActor` as the central mutator. Rejected because `UiActor` already juggles too many concerns and direct mutation there is what caused the current spread.
