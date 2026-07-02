# Round 2 — Keystone Tasks and Unlock Order

## Definition

A **keystone task** is one whose completion removes blockers from a large cluster of downstream work. Pareto rule: prefer keystones over peripheral cleanups.

## Current top keystones

| Rank | Task | Status | Unlocks |
|------|------|--------|---------|
| 1 | `wire-rmcp-client-or-remove-mcp-config.md` | todo | All MCP tasks, subagent/MCP comparison tasks, permission-annotation tasks |
| 2 | `create-grok-build-fixture-recorder-and-record-fixtures.md` + `prepare-grok-build-reference-for-comparison.md` | todo | All `compare-*.md` tasks, findings report, provider/TUI translator tasks |
| 3 | `finish-replacing-custom-tui-widgets.md` | todo | TUI simplification, TUI comparison tasks, layout cleanup |
| 4 | `feature-gate-heavy-runie-core-subsystems.md` | todo | Faster builds, optional subsystems, smaller binary |
| 5 | `stop-flattening-provider-errors-into-strings.md` | todo | Typed error events, better diagnostics, retry logic |
| 6 | `restructure-runieerror-with-typed-variants.md` | todo | Central error handling across crates |
| 7 | `add-tracing-to-runie-provider.md` + `instrument-actor-handlers-with-tracing.md` | todo | Observable async flow, debugging, live diagnostics |
| 8 | `centralize-provider-http-timeouts-and-retry-constants.md` | todo | Provider reliability, retry policy, `fetch_docs` routing |
| 9 | `fix-env-lock-isolation-and-remove-duplicates.md` | todo | Deterministic tests, faster CI, reliable env-dependent tests |
| 10 | `move-tunable-values-from-constants-to-config.md` | todo | User-customizable timeouts/limits, cleaner constants |

## Why these first

- **MCP decision** gates a whole subsystem. Half-implemented MCP config is worse than no MCP.
- **Grok baseline** gates objective comparison. Without recorded fixtures, `compare-*` tasks are subjective.
- **TUI widget replacement** deletes the most custom code and unblocks TUI event-routing cleanup.
- **Feature flags** make every subsequent compile/test cycle faster.
- **Typed errors + tracing** make every subsequent debugging session shorter.

## What to defer

- File/module splitting tasks until internals are simplified.
- String-centralization until UI copy is stable.
- New provider abstractions until existing provider stack is centralized.
- Additional architecture reviews until this backlog is executed.
