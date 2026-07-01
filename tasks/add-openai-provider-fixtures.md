# Add OpenAI provider fixtures

## Status

`done`

## Context

The fixture corpus only covers MiniMax (`runie-testing/src/fixtures/minimax/`). There are no canonical OpenAI text/reasoning/tool/error fixtures.

## Goal

Add `runie-testing/src/fixtures/openai/` with recorded SSE streams for:
- simple text delta
- reasoning content
- parallel tool calls
- rate-limit error

## Acceptance Criteria

- [x] Add sanitized fixture files.
- [x] Add loader similar to `fixtures/minimax.rs`.
- [x] Add at least one agent test using OpenAI fixtures.
- [x] No real API keys in fixtures.

## Design Impact

No change to TUI element design or composition. Only test fixtures change.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** New OpenAI fixture tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
