# Round 5 — Integrated Execution Plan

## Phase 0 — Resolve contradictions and merge duplicates (this week)

1. `resolve-sqlite-vs-jsonl-persistence-conflict.md` — mark SQLite task `wontfix`.
2. `update-providerprotocol-doc-to-reflect-wontfix-decision.md`.
3. `merge-duplicate-tui-widget-replacement-tasks.md`.
4. `merge-duplicate-session-persistence-tasks.md`.
5. `create-unified-architecture-backlog-execution-task.md` — replace per-pass roadmap tasks.
6. Add `blocked_reason` and `supersedes` fields to task template.

## Phase 1 — Stabilize CI and build (week 1)

1. `fix-env-lock-isolation-and-remove-duplicates.md`
2. `gate-test-support-with-cfg-test.md`
3. `add-in-memory-backends-for-unit-tests.md`
4. `eliminate-real-sleeps-in-provider-tests.md`
5. `remove-unused-workspace-dependencies.md`
6. `fix-unix-only-dependencies-in-runie-core.md`
7. `add-features-to-runie-provider.md`

## Phase 2 — Centralize provider stack (week 2)

1. `centralize-provider-http-timeouts-and-retry-constants.md`
2. `centralize-provider-error-status-classification.md`
3. `unify-sse-parsing-on-openai-frame.md`
4. `use-retryconfig-in-with-retry-or-remove-it.md`
5. `use-untagged-enum-for-provider-error-bodies.md`
6. `add-tracing-to-runie-provider.md`
7. `route-fetch-docs-through-central-http-client.md`

## Phase 3 — Decide and implement MCP (week 3)

1. `spike-rmcp-feasibility-before-mcp-decision.md`
2. `wire-rmcp-client-or-remove-mcp-config.md`

## Phase 4 — Simplify TUI (week 4)

1. `spike-tui-textarea-parity-before-widget-replacement.md`
2. `route-tui-autocomplete-through-inputactor-events.md`
3. `route-permission-clearance-through-permissionactor.md`
4. `deduplicate-input-event-mapping-between-forwarder-and-uiactor.md`
5. `finish-replacing-custom-tui-widgets.md`
6. `use-textwrap-for-blockquote-word-wrap.md`
7. `fix-throbber-inversion-and-use-throbber-widgets-tui.md`
8. `derive-agent-running-flag-from-turnstate-events.md`

## Phase 5 — Telemetry and diagnostics (week 5)

1. `instrument-actor-handlers-with-tracing.md`
2. `replace-eprintln-println-with-tracing.md`
3. `add-json-file-logging-for-tui.md`

## Phase 6 — Error typing (week 6)

1. `stop-flattening-provider-errors-into-strings.md`
2. `restructure-runieerror-with-typed-variants.md`
3. `replace-production-expect-panics-with-result-propagation.md`

## Phase 7 — Feature flags and core cleanup (week 7)

1. `feature-gate-heavy-runie-core-subsystems.md`
2. `break-runie-testing-dev-dependency-cycle.md`
3. `move-tunable-values-from-constants-to-config.md`

## Phase 8 — Grok comparison baseline (week 8)

1. `create-grok-build-fixture-recorder-and-record-fixtures.md`
2. `prepare-grok-build-reference-for-comparison.md`

## Phase 9 — Remaining backlog (ongoing)

- Magic numbers cleanup tasks.
- Module split tasks (after simplification).
- String centralization.
- All `compare-*.md` tasks.

## Stopping condition

Stop starting new review rounds after Phase 0. The next work should be implementation, not more horizontal review.

## Success metrics

- `tasks/index.json` has no contradictory `todo` tasks.
- Every `blocked` task has `blocked_reason`.
- CI passes before each phase merges.
- Net LOC decreases or stays flat each phase.
