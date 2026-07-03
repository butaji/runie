# Replace think-block filter with regex

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: none

## Description

The `<think>` block filter uses custom streaming string matching that is fragile with nested or unclosed tags. Replace it with a robust `regex`-based filter that strips think blocks after the markdown stream is processed.

## Acceptance Criteria

- [x] Think tags (`<think>...</think>`) are stripped by a compiled `regex::Regex`.
- [x] Nested and unclosed tags are handled gracefully (strip until EOF for unclosed opening tag).
- [x] The custom streaming matcher is deleted.
- [x] Stripping happens in one place after markdown processing.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `regex_strips_think_blocks` — valid blocks are removed.
- [x] `regex_handles_unclosed_think` — unclosed `<think>` strips to end of input.
- [x] `regex_preserves_text_without_tags` — text without think tags is unchanged.

### Layer 2 — Event Handling
- [x] N/A — filtering is stream/text transform.

### Layer 3 — Rendering
- [x] `think_blocks_not_rendered` — a `TestBackend` buffer shows no think content.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `minimax_inline_think_renders_visible_response` — a captured provider stream containing think tags renders correctly end-to-end.

## Files touched

- `crates/runie-core/src/think.rs`
- `crates/runie-core/src/markdown.rs`
- `crates/runie-provider/src/*.rs`

## Notes

- Consider combining with the shared markdown event stream so think tags are treated as raw HTML and dropped there.
- If model-specific tags differ, keep the regex pattern in the model catalog.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
