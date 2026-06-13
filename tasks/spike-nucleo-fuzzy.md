# Spike: Replace Custom Fuzzy Matching with `nucleo`

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

**Depends on**: crate-replacement-audit

## Description

Evaluate whether `nucleo` (Helix editor's fuzzy matcher) can replace the
custom fuzzy matcher in `crates/runie-core/src/fuzzy.rs`. `nucleo` is not
well-indexed by Context7, but it exists on crates.io as `nucleo = "0.5.0"`.

## Acceptance Criteria

- [ ] Create a throwaway spike branch.
- [ ] Add `nucleo = "0.5"` to `crates/runie-core/Cargo.toml`.
- [ ] Implement `fuzzy_filter_nucleo(query, candidates, limit) -> Vec<&str>`.
- [ ] Compare output ranking with the existing custom matcher on:
  - Command palette items
  - Model selector items
  - @-file references
- [ ] Measure performance on a list of 1000+ candidates.
- [ ] Decide **Adopt** or **Keep Custom** and document the decision in
  `docs/CRATE_DECISIONS.md`.

## Tests

### Layer 1 — State/Logic
- [ ] `nucleo_ranks_exact_match_first`
- [ ] `nucleo_matches_query_chars_in_order`
- [ ] `nucleo_performance_under_10ms_for_1k_items`

## Notes

**Files touched (spike only):**
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/fuzzy.rs`

**Out of scope:**
- This is a spike; do not merge unless the decision is **Adopt**.
