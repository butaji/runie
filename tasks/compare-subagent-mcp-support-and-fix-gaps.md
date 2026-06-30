# Compare subagent and MCP support and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: build-runie-vs-grok-build-comparison-harness
**Blocks**: none

## Description

Grok Build advertises parallel subagents and MCP server support. Runie has subagent/MCP scaffolding that may be incomplete or dead. Compare the documented/reference behavior and decide what to implement, remove, or document as out of scope.

## Scenario Set

1. Grok Build uses subagents for a multi-file refactor.
2. Runie subagent invocation (if any).
3. Grok Build MCP server integration.
4. Runie MCP feature flag / scaffolding.

## Acceptance Criteria

- [ ] Inventory existing Runie subagent/MCP code.
- [ ] Compare with Grok Build's documented behavior.
- [ ] For each gap, create a task (implement or delete dead scaffolding) with unit + E2E AC.
- [ ] If a feature is intentionally out of scope, document the decision in the comparison report.
- [ ] `cargo test --workspace` passes after cleanup.

## Tests

### Layer 1 — State/Logic
- [ ] `mcp_feature_flag_gated_or_removed` — no unreachable MCP code in production builds.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — comparison is architectural; tests defined in child tasks.

## Files touched

- `crates/runie-core/src/actors/subagent/`
- `crates/runie-core/src/mcp/`
- `crates/runie-core/src/proto/`

## Fixture / Replay Strategy

Use recorded Grok Build headless/TUI fixtures for subagent and MCP behavior (or published documentation where live recording is impractical). Runie tests validate architectural decisions against these fixtures; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- May overlap with existing tasks `delete-or-fix-dead-mcp-feature-flag` and `implement-or-remove-mcp-runtime-scaffolding`.
