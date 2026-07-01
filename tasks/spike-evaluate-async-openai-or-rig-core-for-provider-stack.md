# Spike: evaluate async-openai or rig-core for the provider stack

## Status

`todo`

## Context

The OpenAI-compatible provider stack in `crates/runie-provider/src/openai/{protocol,request,stream,normalize}.rs` is ~1,500 lines of hand-rolled SSE parsing, delta handling, and request building. `async-openai` and `rig-core` are mature crates that handle this.

## Goal

Spike replacing the OpenAI provider stack with `async-openai` or `rig-core` behind a thin adapter that converts the crate's stream into Runie's `ProviderEvent`s. The spike should answer: Does the crate support MiniMax/OpenAI-compatible endpoints, reasoning/thinking content, tool-call streaming, and stop-reason mapping?

## Acceptance Criteria

- [ ] Create a branch/spike crate or module using `async-openai` or `rig-core`.
- [ ] Implement a thin adapter from crate streams to `ProviderEvent`.
- [ ] Run the existing provider-replay E2E fixtures against the adapter.
- [ ] Document gaps (reasoning content, custom endpoints, tool-call quirks, dependency conflicts).
- [ ] Decide whether to migrate, hybridize, or keep the custom stack.

## Design Impact

No change to TUI element design or composition. Only the provider implementation changes; the `ProviderEvent` contract stays.

## Tests

- **Layer 1 — State/Logic:** Adapter unit tests for delta → event mapping.
- **Layer 2 — Event Handling:** `ProviderActor` handles adapter events.
- **Layer 3 — Rendering:** N/A for spike.
- **Layer 4 — E2E:** All provider-replay fixtures pass with the adapter.
- **Live tmux testing session (required):** If the spike lands, run a live MiniMax/mock turn end-to-end.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
