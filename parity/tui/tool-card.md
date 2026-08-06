# Tool card

## Anatomy

Action glyph/name, arguments, running spinner, success/error marker, and
structured output rows. The card is keyed by `tool_call_id`.

## States

`running → success | error → collapsed | expanded`.

## Reference

Grok: `src/scrollback/blocks/tool/*` and `scrollback/entry.rs`.
Runie: `LineKind::{Tool,ToolOutput,ToolResult}` and the event renderer.

## Acceptance

`visual-tool-structured.yaml`, `visual-tool-error.yaml`, and parallel-order
core replay tests.
