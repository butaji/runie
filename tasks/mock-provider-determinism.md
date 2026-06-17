# Make MockProvider Delays Deterministic

**Status**: todo
**Milestone**: R3
**Category**: Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`MockProvider` uses `rand::random` for delays, making tests with `MockProvider::with_delay` non-deterministic.

## Acceptance Criteria

- [ ] Accept a seed or use a deterministic RNG for test delays.
- [ ] Tests using `MockProvider::with_delay` produce consistent timing.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `mock_provider_delay_is_deterministic` — same seed yields same delay sequence.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-provider/src/mock.rs`

## Notes

Small test-only improvement; low risk.
