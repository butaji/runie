# Adopt `tiktoken-rs` for Token Estimation

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

**Depends on**: crate-replacement-audit, model-capability-flags

## Description

Replace the `chars/4` token approximation in `crates/runie-core/src/tokens.rs`
with `tiktoken-rs` for OpenAI-compatible models. Keep the approximation as a
fallback for unknown providers. Context7 ID: `/zurawiki/tiktoken-rs`.

## Acceptance Criteria

- [ ] Add `tiktoken-rs = "0.6"` to `crates/runie-core/Cargo.toml`.
- [ ] Create a `Tokenizer` enum:
  ```rust
  pub enum Tokenizer {
      Tiktoken(String), // e.g. "cl100k_base", "o200k_base"
      Approximate,      // chars/4 fallback
  }
  ```
- [ ] Map provider/model to tokenizer name in `ModelCapabilities`.
- [ ] `estimate_tokens(text, tokenizer)` returns accurate count when a tiktoken
  tokenizer is available, otherwise approximation.
- [ ] `TokenTracker` uses the new estimator.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tiktoken_counts_openai_model` — known string produces expected token count.
- [ ] `approximate_fallback_for_unknown_model` — falls back to chars/4.
- [ ] `token_tracker_uses_real_counts` — tracker updates with accurate counts.

## Notes

**ctx7 snippet:**
```rust
use tiktoken_rs::cl100k_base;
let bpe = cl100k_base()?;
let tokens = bpe.encode_with_special_tokens(text);
let count = tokens.len();
```

**Files touched:**
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/tokens.rs`
- `crates/runie-core/src/model_catalog.rs`

**Out of scope:**
- Anthropic / Gemini native tokenizers (use approximation until crates emerge).
