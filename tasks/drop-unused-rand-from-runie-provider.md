# Drop unused `rand` from `runie-provider`

**Status**: todo
**Milestone**: R6
**Category**: Dependencies
**Priority": P2

**Depends on**: none
**Blocks**: none

## Description

`rand` is declared in `runie-provider` and used only for `MockProvider::random_delay`. Make mock delays deterministic and remove the dependency.

## Acceptance Criteria

- [ ] Replace `MockProvider::random_delay` with a deterministic delay strategy (e.g., fixed or stepped).
- [ ] Remove `rand` from `crates/runie-provider/Cargo.toml`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `mock_delay_is_deterministic` — delay is predictable given inputs.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-provider/Cargo.toml`
- `crates/runie-provider/src/mock.rs`

## Notes

- AGENTS.md forbids artificial delays in automatic tests; mock provider delays should not affect deterministic tests.
