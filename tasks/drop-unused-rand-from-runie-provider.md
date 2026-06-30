# Drop unused `rand` from `runie-provider`

**Status**: done
**Milestone**: R6
**Category**: Dependencies
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`rand` was previously a dependency of `runie-provider`, used only for `MockProvider::random_delay`. It has since been replaced with a custom `xorshift64star` implementation — no `rand` crate is present in the workspace.

## Acceptance Criteria

- [x] Replace `MockProvider::random_delay` with a deterministic delay strategy (e.g., fixed or stepped).
- [x] Remove `rand` from `crates/runie-provider/Cargo.toml`. — **Already done**: `rand` is not listed; `mock.rs` uses a hand-rolled `xorshift64star` seeded deterministically.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mock_provider_delay_is_deterministic` — delay is predictable given inputs (`crates/runie-provider/src/tests.rs`).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- None — the migration was already completed before this task was authored.

## Notes

- AGENTS.md forbids artificial delays in automatic tests; `MockProvider::random_delay` is seeded with a fixed value so tests remain deterministic.
- `MockProvider::default()` uses seed `42`; `MockProvider::with_delay(min, max)` also uses seed `42`; `MockProvider::with_seed(min, max, s)` accepts an explicit seed.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
