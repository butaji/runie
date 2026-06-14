# Architecture Decision Records

ADRs document the design decisions that shaped runie. Each one captures the
context, options considered, and the decision made. The ones numbered
0001-0010 were written for an early actor-based architecture that was
superseded by the simpler MVU pattern now in use; they are kept in
`archive/` for historical reference.

## Current architecture (in use)

| ADR | Decision |
|-----|----------|
| [0011](0011-non-interactive-modes-separate-binaries.md) | `runie-print` / `runie-json` / `runie-server` are separate binaries |
| [0012](0012-streaming-emits-event-per-chunk.md) | LLM chunks → individual events → accumulated into the response |
| [0013](0013-configurable-keybindings.md) | Keybindings loaded from JSON, hot-reloadable |
| [0014](0014-theme-system-opaline.md) | Theme engine via opaline (39 themes + custom TOML) |
| [0015](0015-command-registry-and-palette.md) | Command registry, DSL, palette, dialog forms |
| [0016](0016-unified-action-system.md) | ItemAction enum unifying panel item behavior |
| [0017](0017-actor-runtime-and-event-bus.md) | Lightweight tokio-task actors + typed Actor trait + EventBus, no external actor framework |
| [0018](0018-provider-llm-event-normalization.md) | All providers emit a normalized `LLMEvent` stream |
| [0019](0019-tool-registry-and-mcp.md) | Tool trait registry, permission rulesets, and in-house MCP client |
| [0020](0020-team-mode-orchestration.md) | Solo/Team execution modes and Orchestrator-Harness Protocol |

## For the current architectural overview

See [`../SPEC.md`](../SPEC.md).

## How to write a new ADR

1. Copy the next number (`0017-…`)
2. Use the format: `# Title`, then `## Context`, `## Decision`, `## Consequences`
3. Keep it short — ADRs that exceed 50 lines are usually too broad and should
   be split
