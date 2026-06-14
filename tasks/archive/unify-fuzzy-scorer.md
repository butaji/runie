# Unify Fuzzy Matching Scorers

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

## Description

`runie-core` maintains two independent fuzzy matchers:

- `crates/runie-core/src/fuzzy.rs::fuzzy_match` — used for `@-file` references.
- `crates/runie-core/src/dialog/score.rs::match_score`/`fuzzy_score` — used for panel
  filtering.

Both implement ordered-character matching with start/word-boundary bonuses and gap
penalties. They should be merged into one crate-internal scorer.

## Acceptance Criteria

- [x] A single fuzzy scorer exists (`runie_core::fuzzy::score`).
- [x] `@-file` filtering and panel filtering both use it.
- [x] Existing behavior is preserved: all current `@-ref` and command-palette tests still
  pass with the same top results for representative queries.
- [x] `crates/runie-core/src/dialog/score.rs` is reduced to a thin wrapper.

## Tests

### Layer 1 — State/Logic
- [x] `fuzzy_score_exact_match_beats_partial`.
- [x] `fuzzy_score_start_bonus` — matches at the start of a word score higher.
- [x] `panel_filter_and_at_ref_agree_on_order` — same query produces the same relative
  ordering in both callers.

### Layer 2 — Event Handling
- [ ] No event changes.

### Layer 3 — Rendering
- [ ] No rendering changes.

## Files touched

- `crates/runie-core/src/fuzzy.rs`
- `crates/runie-core/src/dialog/score.rs`
- `crates/runie-core/src/dialog/mod.rs` or callers that import `score`
- `crates/runie-core/src/update/at_refs.rs`
- `crates/runie-core/src/tests/fuzzy.rs`
- `crates/runie-core/src/dialog/tests.rs`

## Out of scope

- Replacing the scorer with `nucleo` (covered by `tasks/spike-nucleo-fuzzy.md`).
