# Replace token chars/4 heuristic with tiktoken-rs

## Status

`todo`

## Context

`crates/runie-core/src/tokens.rs` counts tokens as `chars().count().div_ceil(4)`. This is inaccurate for non-Latin text and code. If token counts are used for truncation or cost estimates, the error is material.

## Goal

Adopt `tiktoken-rs` for OpenAI-compatible token counts, or `tokenizers` for model-specific counts. Keep the old heuristic only as a documented fallback for quick UI estimates if exact tokenization is unavailable.

**Design impact:** No change to TUI element design or composition. Only the numeric token count rendered in the status bar may become more accurate; the visual style remains the same.

## Acceptance Criteria

- [ ] Add `tiktoken-rs` (or `tokenizers`) to workspace dependencies.
- [ ] Implement token counting per message and per conversation for supported models.
- [ ] Fall back to chars/4 only for unknown model families, with a clear log.
- [ ] Update call sites in truncation/cost estimation to use the new API.

## Tests

- **Layer 1 — State/Logic:** Unit tests comparing token counts for English, code, and CJK text against known values.
- **Layer 1:** Unknown model uses fallback heuristic and emits a warning.
- **Layer 2 — Event Handling:** Token count facts are emitted after each streaming delta.
- **Layer 3 — Rendering:** `TestBackend` shows token count within a reasonable tolerance.
- **Layer 4 — E2E:** Provider replay fixture returns expected token counts for a known prompt.
- **Live tmux validation:** Start a turn with a non-trivial prompt; the token count in the status bar matches expectations for the model.
