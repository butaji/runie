# Replace streaming-buffer classifier with pulldown-cmark

## Status

`todo`

## Context

`crates/runie-core/src/streaming_buffer.rs:89-205` contains a custom state machine that classifies lines as plain, fence, or table-separator to decide which streamed markdown lines are “stable” enough to flush. This duplicates information already available from `pulldown-cmark` events.

## Goal

Replace the fence/table classifier with a `pulldown-cmark` event stream over a sliding window, or remove the buffer entirely if the renderer can handle partial input. Keep the same stable-line semantics for scroll math.

## Acceptance Criteria

- [ ] Remove the custom fence/table classifier.
- [ ] Implement stable-line detection via `pulldown-cmark` events or `tui-markdown` partial rendering.
- [ ] Line-count and scroll math remain identical for all streaming test fixtures.
- [ ] Chunk-boundary behavior is preserved.

## Design Impact

No change to TUI element design or composition. Only the internal streaming-stability logic changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for stable-line detection on streamed chunks.
- **Layer 2 — Event Handling:** Streaming deltas produce the same `MessageDelta` facts.
- **Layer 3 — Rendering:** `TestBackend` snapshots across chunk boundaries match existing snapshots.
- **Layer 4 — E2E:** Provider replay fixture streams a fenced code block split across many chunks.
- **Live tmux testing session (required):** Start a turn that returns a large markdown response; verify no visual jumps or mis-rendered fences.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
