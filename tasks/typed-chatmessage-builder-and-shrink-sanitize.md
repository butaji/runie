# Typed ChatMessage builder and shrink sanitize.rs

## Status

`todo`

## Context

`crates/runie-core/src/sanitize.rs` (333 LOC) is a post-hoc fix-up pipeline for `ChatMessage` sequences (remove empty assistant messages, dangling tool calls, orphan tool results, merge consecutive same-role messages, etc.). Many of these issues can be prevented by a typed builder and stronger construction rules. `crates/runie-core/src/tokens.rs` also uses a `chars/4` heuristic.

## Goal

Introduce a `ChatMessage` builder that enforces valid sequences at construction time, then shrink `sanitize_messages` to validation-only. Keep `tiktoken-rs` for accurate token counts.

## Acceptance Criteria

- [ ] Add a `ChatMessageBuilder` or constructor helpers that prevent invalid sequences.
- [ ] Remove fix-up logic from `sanitize.rs`; keep only assertions/normalization.
- [ ] Update callers (agent loop, headless, subagent) to use the builder.
- [ ] All provider-replay tests pass.

## Design Impact

No change to TUI element design or composition. Only message construction/sanitization behavior changes.

## Tests

- **Layer 1 — State/Logic:** Builder unit tests for invalid sequences and token counting.
- **Layer 2 — Event Handling:** Message events carry valid sequences.
- **Layer 3 — Rendering:** Message list rendering is unchanged.
- **Layer 4 — E2E:** Provider replay fixture with tool calls produces a valid final conversation.
- **Live tmux validation:** Multi-tool conversation renders without sanitization artifacts.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
