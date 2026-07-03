# Fix provider feature flags so mock provider does not leak into production binaries

**Status**: done
**Milestone**: R7
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`runie-provider/Cargo.toml` includes `mock` in its default features, and `runie-tui/Cargo.toml` depends on `runie-provider` without `default-features = false`. As a result, the production TUI binary compiles in the mock provider, which activates whenever `RUNIE_MOCK` or `RUNIE_MOCK_DELAY` is set. This increases binary size and attack surface for test-only code.

## Acceptance Criteria

- [x] Remove `mock` from `runie-provider` default features.
- [x] Make `runie-tui` and `runie-cli` declare only the provider features they need.
- [x] Ensure tests and `runie-testing` can still opt into the mock provider.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `mock_provider_requires_explicit_feature` — building `runie-provider` without features excludes `MockProvider`.

### Layer 2 — Event Handling
- [x] N/A — feature-flag concern.

### Layer 3 — Rendering
- [x] N/A — feature-flag concern.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `mock_provider_tests_still_pass` — `runie-testing` and replay tests build with the mock feature enabled.

### Live Tmux Testing Session
- [x] Build the release TUI binary and confirm `RUNIE_MOCK=1` does not activate the mock provider.

## Files touched

- `crates/runie-provider/Cargo.toml`
- `crates/runie-tui/Cargo.toml`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-testing/Cargo.toml`

## Notes

- Related to `audit-mock-provider-delay-constants.md`; consider doing both together.
