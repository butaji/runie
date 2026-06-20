# Reconsider tiktoken-rs token estimation

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`tiktoken-rs` was adopted in the done task `adopt-tiktoken-rs` for accurate BPE token counts. Reversal argument under the YAGNI / stdlib posture:

- Only one consumer: `crates/runie-core/src/tokens.rs` (with `estimate_tokens_for_model` / `estimate_tokens_with_tokenizer` / `token_tracker_for`).
- The crate already has a heuristic `estimate_tokens` path (chars/4-style); most UI surfaces (token counter, truncation) tolerate ±20%.
- `tiktoken-rs` pulls in `regex` + `fancy-regex` + a BPE merge table (~1.5MB embedded) and runs a real tokenizer per call.
- For a coding agent that already receives `usage` from the provider on every response, the local estimate is a fallback, not the source of truth.

Either (a) drop `tiktoken-rs` and rely on the heuristic + provider-reported usage, (b) gate it behind a `tiktoken` feature (off by default), or (c) keep and document a concrete accuracy requirement.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Drop** — `tokens.rs` uses only the heuristic estimator; `tiktoken-rs` removed from `runie-core/Cargo.toml` and `[workspace.dependencies]`; `Tokenizer::Tiktoken` variant removed; OR
  - (b) **Feature-gate** — `tiktoken` feature added, `tiktoken-rs` optional, default build uses heuristic; OR
  - (c) **Keep + document** — a concrete accuracy requirement (e.g. truncation cutoff must be within 5% of real token count) is written into `tokens.rs` module docs.
- [ ] If (a) or (b): default `cargo build --workspace` no longer pulls `tiktoken-rs`, `regex`, `fancy-regex`, or the BPE merge table.
- [ ] `TokenTracker` still produces a non-zero estimate for any non-empty input.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `heuristic_estimate_within_30pct_of_chars` — `estimate_tokens("hello world")` returns a value in the expected band for a 11-char string.
- [ ] `token_tracker_for_uses_heuristic_by_default` — `token_tracker_for(...)` with the feature off returns a `TokenTracker` backed by the heuristic.
- [ ] `estimate_tokens_for_model_falls_back_to_heuristic` — when the tiktoken path is unavailable (feature off), the model-specific estimator still returns a value.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [ ] `token_counter_renders_nonzero` — the status bar token counter still renders a non-zero count for a populated session.

### Layer 4 — Smoke / Crash
- [ ] `smoke_default_build_excludes_tiktoken` — `cargo build --workspace` does not pull `tiktoken-rs`.
- [ ] `smoke_truncation_still_fires` — a session with a very large message still triggers truncation under the heuristic estimator.

## Files touched

- `crates/runie-core/src/tokens.rs` (drop tiktoken path or gate it)
- `crates/runie-core/Cargo.toml` (remove / optional-ize `tiktoken-rs`)
- `Cargo.toml` (remove from `[workspace.dependencies]`)

## Notes

This is the most accuracy-sensitive drop in the audit. If truncation or cost-estimation features depend on tight token budgets, option (c) is the safe choice. The provider-reported `usage` field on responses is always authoritative for billing; the local estimator only matters for pre-flight truncation decisions. If option (c), link justification and close as `wontfix`.
