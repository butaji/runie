# Add Grok TUI keystroke replay and tool mapping

## Status

`todo`

## Context

Grok TUI scenarios are captured as tmux pane dumps; there is no bridge to Runie `TestBackend` events or canonical tool-name mapping.

## Goal

Add a keystroke DSL → Runie event translator and a Grok→Runie tool alias map for deterministic TUI comparisons.

## Acceptance Criteria
- [ ] Define small keystroke DSL.
- [ ] Translate DSL to `runie_core::Event`s.
- [ ] Add tool-name/schema aliases.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for DSL translation and aliases.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** `TestBackend` scenario matches captured pane.
- **Layer 4 — E2E:** TUI comparison scenario passes.
- **Live tmux testing session (required):** Re-recorded scenario matches live behavior.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (DSL translation is a utility; actors remain authoritative).
- [ ] **Trigger events:** DSL keystrokes translate to `Event` variants.
- [ ] **Observer events:** N/A (translation doesn't emit events).
- [ ] **No direct mutations:** DSL translation doesn't mutate actor state.
- [ ] **No new mirrors:** Tool aliases are mappings, not authoritative state.
- [ ] **Async work observed:** N/A (synchronous translation).
