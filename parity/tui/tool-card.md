# Tool card

## Anatomy

Action glyph/name, arguments, running spinner, success/error marker, and
structured output rows. The card is keyed by `tool_call_id`.

## States

`running → success | error → collapsed | expanded`.

## Reference

Grok: `src/scrollback/blocks/tool/*` and `scrollback/entry.rs`.
Runie: `LineKind::{Tool,ToolOutput,ToolResult}` and the event renderer.

Background subagents use the typed `AgentEvent` lifecycle family
(`BackgroundWorkStarted`, `BackgroundWorkProgress`, `BackgroundWorkFinished`,
and `BackgroundWorkCancelled`). Terminal rows preserve elapsed time and
failure detail; running rows use actor-owned `ToolRunning` animation demand.
This is intentionally separate from generic tool execution state.

## Acceptance

`visual-tool-structured.yaml`, `visual-tool-error.yaml`, and parallel-order
core replay tests. `visual-background-work.yaml` covers the subagent lifecycle
and deterministic terminal labels.
