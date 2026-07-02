# Fold lifecycle state machine into provider events

## Status

`todo`

## Context

`crates/runie-core/src/lifecycle.rs` (167 LOC) tracks open text/thinking blocks and synthesizes `Start`/`End` lifecycle events. This is duplicative when the provider layer could emit the same events directly.

## Goal

Remove `LifecycleState` and have the provider streaming layer emit the equivalent `TextStarted`/`TextEnded`/`ThinkingStarted`/`ThinkingEnded` facts. If a provider library (`async-openai`/`rig-core`) emits content deltas, map them directly.

## Acceptance Criteria

- [ ] Delete `lifecycle.rs` or reduce it to a small mapping helper.
- [ ] Provider/normalizer emits start/end events for text and thinking blocks.
- [ ] TUI behavior for streaming/thinking indicators is unchanged.
- [ ] All lifecycle tests pass.

## Design Impact

No change to TUI element design or composition. Only event-generation behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for provider deltas → start/end events.
- **Layer 2 — Event Handling:** `UiActor` receives the same lifecycle facts.
- **Layer 3 — Rendering:** `TestBackend` thinking indicators and turn-complete behavior match.
- **Layer 4 — E2E:** Provider replay fixture emits correct lifecycle events.
- **Live tmux testing session (required):** Start a turn with thinking blocks; the thinking indicator appears and disappears correctly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ProviderActor` owns the streaming state machine; lifecycle tracking moves from `LifecycleState` to provider.
- [ ] **Trigger events:** `ProviderEvent` variants (`TextDelta`, `ThinkingDelta`) trigger lifecycle tracking.
- [ ] **Observer events:** `TextStart`, `TextEnd`, `ThinkingStart`, `ThinkingEnd` notify observers.
- [ ] **No direct mutations:** Provider emits lifecycle events; no direct `AppState` mutation.
- [ ] **No new mirrors:** `LifecycleState` is removed; lifecycle tracking becomes a pure function in provider.
- [ ] **Async work observed:** Provider streaming is already observed via `ProviderEvent` channel.
