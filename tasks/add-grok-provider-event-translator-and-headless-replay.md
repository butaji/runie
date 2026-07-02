# Add Grok provider-event translator and headless replay

## Status

`todo`

## Context

Grok headless output is JSONL, not SSE; there is no path to turn it into Runie `ProviderEvent`s or replay it through the headless CLI.

## Goal

Build a translator from Grok JSONL to `ProviderEvent`s and a headless replay harness using `runie-agent::run_headless_cli`.

## Acceptance Criteria
- [ ] Parse Grok JSONL deltas/tool-calls/usage/errors.
- [ ] Emit canonical `ProviderEvent`s.
- [ ] Inject replay provider into headless CLI path.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for translator.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless comparison scenario passes with Grok fixture.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ProviderActor` owns provider state; translator emits events through it.
- [ ] **Trigger events:** Translator parses JSONL and emits `ProviderEvent` variants.
- [ ] **Observer events:** `ProviderEvent` variants notify `ProviderActor`.
- [ ] **No direct mutations:** Translator must emit events, not directly mutate state.
- [ ] **No new mirrors:** Translator is a conversion layer, not a state store.
- [ ] **Async work observed:** Translator processes JSONL synchronously; no new async work.
