# Replace think-block filter with regex

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: unify-markdown-processing-around-pulldown-cmark
**Blocks**: none

## Description

The `<think>` block filter uses custom streaming string matching that is fragile with nested or unclosed tags. Replace it with a robust `regex`-based filter that strips think blocks after the markdown stream is processed.

## Acceptance Criteria

- [ ] Think tags (`<think>...</think>`) are stripped by a compiled `regex::Regex`.
- [ ] Nested and unclosed tags are handled gracefully (strip until EOF for unclosed opening tag).
- [ ] The custom streaming matcher is deleted.
- [ ] Stripping happens in one place after markdown processing.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `regex_strips_think_blocks` — valid blocks are removed.
- [ ] `regex_handles_unclosed_think` — unclosed `<think>` strips to end of input.

### Layer 2 — Event Handling
- [ ] N/A — filtering is stream/text transform.

### Layer 3 — Rendering
- [ ] `think_blocks_not_rendered` — a `TestBackend` buffer shows no think content.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_m3_think_filter` — a captured provider stream containing think tags renders correctly end-to-end.

## Files touched

- `crates/runie-core/src/think.rs`
- `crates/runie-core/src/markdown.rs`
- `crates/runie-provider/src/*.rs`

## Notes

- Consider combining with the shared markdown event stream so think tags are treated as raw HTML and dropped there.
- If model-specific tags differ, keep the regex pattern in the model catalog.
