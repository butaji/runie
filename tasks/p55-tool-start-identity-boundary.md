# p55 — Pi `tool_started` identity boundary

Status: in progress — actor-owned identity, pending ordering, YAML oracle, and
restored-result reservation implemented; persisted pending replay remains open
(2026-08-08)

## First implementation slice (2026-08-08)

`ToolExecutionStart` now crosses the `SessionActor` mailbox as raw provider
facts. When the actor has the assistant/tool context and an active operation,
it derives `assistantEntryId`, `toolIndex`, `runId`, `effectiveArgs`, reserves
the result entry ID, and emits the complete Pi-shaped lane record. A matching
tool result consumes that reservation. Complete records validate assistant
identity, tool index/call identity, result identity, replay policy, and
duplicate invocation.

Out-of-order starts are retained as actor-owned pending facts until a later
assistant entry supplies the context; the bridge no longer constructs or
publishes a partial session record. Pending facts are cleared on reset.
Persisted replay of pending, not-yet-admitted starts remains open because Pi
does not journal an incomplete `tool_started` record. Completed `tool_started`
records that still lack their result entry are replayed into the actor-local
reservation map during import, so a post-restore result keeps its journaled
identity.

`visual-tool.yaml` now asserts the complete tool-start payload through the
runtime YAML state DSL, including actor-issued assistant and result identities.

## Source contract

Pi's `ToolStartedRecord` contains:

- `runId`
- `assistantEntryId`
- `toolIndex`
- `toolCallId`
- `toolName`
- `effectiveArgs`
- `resultEntryId`
- `replay` (`never` or `safe`)

Pi's reducer also validates that the assistant entry exists, the indexed
content block is the matching tool call, and the `(assistantEntryId,
toolIndex)` invocation is not duplicated.

## Current Runie gap

The `new_with_bus` bridge currently turns `ToolExecutionStart` into a generic
`OperationRecordCreated` payload before the session actor sees it. That event
contains only `id`, `toolCallId`, `toolName`, and `args`; it has no actor-owned
assistant identity, tool index, operation ID, result-entry reservation, or
replay policy. Strict validation would therefore drop a live tool event, while
fabricating those fields in the bridge would violate SSOT ownership.

## Required event-driven implementation

1. Add a `Command::ToolStarted` mailbox command carrying only the provider
   event facts (`toolCallId`, `toolName`, `args`) and an acknowledgement.
2. In the `SessionActor` worker, locate the latest assistant entry and the
   matching `ToolCall` index from actor-owned journal state.
3. Reserve the actor-owned result entry identity in the same mailbox turn and
   retain the reservation keyed by `toolCallId` until the matching tool-result
   message is appended. On import, rebuild that operational map from the
   durable `tool_started` records whose result entry is not yet present.
4. Emit/reduce the complete Pi-shaped `tool_started` lane fact from that
   actor-owned data. The event sequence must be
   `ToolExecutionStart → actor reservation → tool_started snapshot`.
5. Consume the reservation when `MessageEnd(ToolResult)` arrives; reject a
   duplicate or unknown tool result without mutating another actor.
6. Validate the full Pi record shape and invocation linkage, then add YAML
   assertions for fields, ordering, and reserved result identity.

## Acceptance evidence

- A YAML event sequence can start a tool, append its result, and assert the
  complete `tool_started` payload without Rust fixture code.
- Parallel tool calls retain distinct actor-issued result identities.
- JSONL round-trip preserves every field and replay policy, and restored
  unsettled starts preserve their reserved result identity.
- The bridge no longer constructs a session lane record directly.
- `just ci` remains green and no test uses `sleep()`.
