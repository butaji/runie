# Centralize test timeouts

## Objective

Define shared timeout constants for the test suite instead of scattering magic
numbers across tests.

## Why this matters

Timeouts currently range from 2 s to 60 s with no documented rationale. This
makes tests hard to tune and produces unpredictable CI behavior.

## Proposed constants

These become fields of the unified `TimeoutConfig` defined in
`tasks/dsl-harness-timeouts.md`:

```rust
pub struct TimeoutConfig {
    pub startup: Duration,   // 5s
    pub response: Duration,  // 10s
    pub dialog: Duration,    // 3s
    pub idle: Duration,      // 1s
    pub build: Duration,     // 300s
}
```

Expose the defaults through `runie_tests::prelude::*` and allow per-test
overrides via the DSL builder.

## Files to update

- DSL library code
- All tests that currently hardcode durations

## Dependencies

- `dsl_harness_timeouts`

## Acceptance checklist

- [ ] No magic timeout durations remain in tests.
- [ ] All constants are documented with rationale.
- [ ] Tests still pass.
