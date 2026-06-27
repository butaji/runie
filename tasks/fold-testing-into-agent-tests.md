# Fold runie-testing into runie-agent tests

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Cancellation Reason

The task description was inaccurate: `runie-testing` has 4 consumers:

1. `crates/runie-agent/src/subagent.rs` — imports `runie_testing::mock_provider`
2. `crates/runie-tui/Cargo.toml` — dev-dependency
3. `crates/runie-agent/Cargo.toml` — dev-dependency
4. `crates/runie-provider/Cargo.toml` — dev-dependency

Per the task's own notes: "if a second consumer is planned, keep the crate." Since there are 4 consumers, the crate should remain as a shared test utility crate.
