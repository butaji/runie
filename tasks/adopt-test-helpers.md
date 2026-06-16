# Adopt Shared Test Infrastructure

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Create shared test helpers in `runie-testing` crate for consistent test infrastructure:

```rust
// TestCodex / TestRunner helper
pub struct TestRunner {
    pub app: TestApp,
    pub test_home: TempDir,
}

impl TestRunner {
    pub async fn new() -> Self;
    pub fn submit(&self, input: &str) -> SubmissionId;
    pub async fn expect_event<F>(&self, predicate: F) -> EventMsg
    where F: Fn(&EventMsg) -> bool;
    pub async fn expect_event_with_timeout<F>(&self, timeout: Duration, predicate: F) -> Option<EventMsg>;
}

// Mock API responses
pub fn ev_response_created() -> Event { ... }
pub fn ev_output_text_delta(text: &str) -> Event { ... }
pub fn ev_completed() -> Event { ... }
pub fn ev_error(message: &str) -> Event { ... }

// Common test fixtures
pub async fn load_default_config_for_test(test_home: &TempDir) -> Config;
pub fn mock_provider() -> MockProvider;

// Skip conditions
#[macro] skip_if_seatbelt!()  // Skip if SEATBELT=1
#[macro] skip_if_integration!()  // Skip slow integration tests
```

Reference: `~/Code/agents/codex-rs/core/tests/common/` and `test_codex.rs`

## Acceptance Criteria

- [ ] `runie-testing` crate created with test infrastructure.
- [ ] `TestRunner` helper for event-driven tests.
- [ ] Mock API response builders.
- [ ] Common fixtures (config, provider, session).
- [ ] Skip macros for conditional tests.
- [ ] `cargo test --workspace` succeeds with shared helpers.

## Tests

### Layer 1 — State/Logic
N/A (this is test infrastructure itself).

### Layer 2 — Event Handling
- [ ] `test_runner_submit_and_expect_event` — integration test pattern works.
- [ ] `mock_provider_handles_stream` — mock streams responses.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-testing/` (new crate)
- `crates/runie-core/tests/` — migrate to use helpers
- `crates/runie-tui/tests/` — migrate to use helpers

## Notes

Reduces test boilerplate across all crates. Enables consistent test patterns.
