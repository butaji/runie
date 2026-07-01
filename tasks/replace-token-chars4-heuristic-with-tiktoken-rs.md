# Replace token chars/4 heuristic with tiktoken-rs

## Status

`done`

## Context

`crates/runie-core/src/tokens.rs` counts tokens as `chars().count().div_ceil(4)`. This is inaccurate for non-Latin text and code. If token counts are used for truncation or cost estimates, the error is material.

## Goal

Adopt `tiktoken-rs` for OpenAI-compatible token counts, or `tokenizers` for model-specific counts. Keep the old heuristic only as a documented fallback for quick UI estimates if exact tokenization is unavailable.

**Design impact:** No change to TUI element design or composition. Only the numeric token count rendered in the status bar may become more accurate; the visual style remains the same.

## Acceptance Criteria

- [x] Add `tiktoken-rs` (or `tokenizers`) to workspace dependencies. (`tiktoken = "3.5"` in workspace Cargo.toml)
- [x] Implement token counting per message and per conversation for supported models. (`tiktoken_count()` function in `tokens.rs`)
- [x] Fall back to chars/4 only for unknown model families, with a clear log. (`chars4_count()` fallback in `estimate_tokens()`)
- [x] Update call sites in truncation/cost estimation to use the new API. (call sites use `estimate_tokens()` which uses tiktoken)

## Tests

- **Layer 1 — State/Logic:** Unit tests comparing token counts for English, code, and CJK text against known values.
- **Layer 1:** Unknown model uses fallback heuristic and emits a warning.
- **Layer 2 — Event Handling:** Token count facts are emitted after each streaming delta.
- **Layer 3 — Rendering:** `TestBackend` shows token count within a reasonable tolerance.
- **Layer 4 — E2E:** Provider replay fixture returns expected token counts for a known prompt.
- **Live tmux validation:** Start a turn with a non-trivial prompt; the token count in the status bar matches expectations for the model.

## Implementation Notes

- `tiktoken = "3.5"` added to workspace dependencies
- `tokens.rs` now uses `tiktoken::get_encoding("cl100k_base")` for OpenAI-compatible providers
- `estimate_tokens()` tries tiktoken first, falls back to `chars4_count()` for unknown providers
- `estimate_tokens_for_model()` specifically checks for `openai` provider to use tiktoken
