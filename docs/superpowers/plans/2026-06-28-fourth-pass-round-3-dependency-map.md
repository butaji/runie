# Round 3 — Dependency Map and Sequencing

## Dependency clusters

### Cluster A: Event vocabulary (foundation)

```
derive-event-taxonomies-with-strum-or-proc-macro
  ├── collapse-intent-into-shared-event-payloads
  ├── collapse-durablecoreevent-into-event-enum
  └── replay-sessions-via-events-through-appstate
```

These are mostly `done`. They enable everything below.

### Cluster B: State ownership (foundation)

```
merge-agentstate-into-turnstate-projection
  ├── remove-direct-appstate-mutation-from-core-update-handlers
  ├── route-permission-clearance-through-permissionactor
  └── derive-agent-running-flag-from-turnstate-events
```

Mostly `done`. Enables clean TUI and actor logic.

### Cluster C: Provider stack

```
centralize-reqwest-client-and-url-normalization
  ├── centralize-provider-error-status-classification
  ├── unify-sse-parsing-on-openai-frame
  ├── use-retryconfig-in-with-retry-or-remove-it
  └── route-fetch-docs-through-central-http-client
      └── add-tracing-to-runie-provider
```

Do these before any new provider abstraction or Grok comparison.

### Cluster D: Tool/MCP

```
generate-tool-dispatch-from-single-registry
  └── wire-rmcp-client-or-remove-mcp-config
        ├── use-mcp-tool-annotations-instead-of-custom-permission-constants
        └── compare-subagent-mcp-support-and-fix-gaps
```

Decision on MCP must happen before any MCP comparison work.

### Cluster E: TUI

```
route-tui-autocomplete-through-inputactor-events
route-permission-clearance-through-permissionactor
derive-agent-running-flag-from-turnstate-events
  └── finish-replacing-custom-tui-widgets
        ├── replace-custom-form-rendering-with-tui-textarea
        └── replace-custom-input-box-with-tui-textarea
```

### Cluster F: Grok comparison

```
create-grok-build-fixture-recorder-and-record-fixtures
prepare-grok-build-reference-for-comparison
  ├── compare-*.md (all comparison tasks)
  └── write-runie-vs-grok-build-findings-report.md
```

### Cluster G: Testing hygiene

```
fix-env-lock-isolation-and-remove-duplicates
  ├── gate-test-support-with-cfg-test
  ├── add-in-memory-backends-for-unit-tests
  └── eliminate-real-sleeps-in-provider-tests
```

### Cluster H: Build/crate hygiene

```
feature-gate-heavy-runie-core-subsystems
  ├── remove-unused-workspace-dependencies
  ├── fix-unix-only-dependencies-in-runie-core
  └── add-features-to-runie-provider
```

## Suggested sequencing

1. Resolve contradictions (SQLite/JSONL, ProviderProtocol doc).
2. Execute Cluster A/B (already mostly done; verify).
3. Execute Cluster G (testing hygiene) to stabilize CI.
4. Execute Cluster H (build hygiene) to speed up cycles.
5. Execute Cluster C (provider stack).
6. Decide and execute Cluster D (MCP).
7. Execute Cluster E (TUI widgets + routing).
8. Execute Cluster F (Grok comparison baseline).
9. Execute remaining peripheral tasks.
