# Unify Token Counting Functions

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

## Description

Two token-counting approximations live in `runie-core`:

- `crates/runie-core/src/tokens.rs::estimate_tokens` uses `chars.div_ceil(4)`.
- `crates/runie-core/src/model.rs::count_tokens` uses `chars / 4`.

They differ only in rounding and are used by different call sites. `estimate_tokens` is
slightly more correct (ceil), so it should become the single function.

## Acceptance Criteria

- [x] `count_tokens` is removed from `crates/runie-core/src/model.rs`.
- [x] All callers use `runie_core::tokens::estimate_tokens`.
- [x] Existing tests in `crates/runie-core/src/tests/token_counters/counters.rs` are
  updated and still pass.
- [x] `cargo build --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `estimate_tokens_one_char_rounds_up` — `"x"` → `1` (ceil behavior).
- [x] `estimate_tokens_four_chars_is_one` — `"test"` → `1`.

### Layer 2 — Event Handling
- [ ] No event changes.

### Layer 3 — Rendering
- [ ] No rendering changes.

## Files touched

- `crates/runie-core/src/model.rs`
- `crates/runie-core/src/update/input.rs`
- `crates/runie-core/src/update/agent.rs`
- `crates/runie-core/src/tests/token_counters/counters.rs`

## Out of scope

- Adopting `tiktoken-rs` (covered by `tasks/adopt-tiktoken-rs.md`).
