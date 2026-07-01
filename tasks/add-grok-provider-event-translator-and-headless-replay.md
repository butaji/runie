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
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
