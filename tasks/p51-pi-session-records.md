# p51 — Pi session configuration records

Status: planned — source contract captured; implementation not yet started

## Source contract

Pi's `packages/agent/src/harness/session/types.ts` defines non-message JSONL
entries which are part of the session journal and therefore are not reducible
to `AgentMessage` values:

- `model_change` (`provider`, `modelId`)
- `thinking_level_change` (`thinkingLevel`)
- `active_tools_change` (`activeToolNames`)

The same file separately defines compaction, branch-summary, custom entries,
and operation-lane records. Those require additional state and are deliberately
not silently represented by this task.

## Runie gap

`runie-core::SessionSnapshot` currently models only the message lane. The
`SessionActor` receives `MessageEnd` and reset events, but configuration events
are not journaled or emitted by `to_jsonl`. Treating them as synthetic messages
would violate Pi's wire contract and the actor SSOT rule.

## Implementation boundary

1. Add a renderer-independent `SessionRecord` sum type with the three
   configuration variants and the existing message variant.
2. Preserve one ordered sequence/parent/leaf invariant across all records;
   message-only `entries` remains a compatibility projection until callers are
   migrated.
3. Add explicit actor mailbox commands generated from `AgentEvent::ModelChanged`
   and `ThinkingLevelChanged`; active tool changes need a typed core event before
   the session actor can reduce them.
4. Extend validated JSONL v4 import/export for these three record types.
5. Add runtime YAML assertions for ordered record kinds and payloads; no
   fixture-specific Rust scenario code.

## Acceptance

- A YAML event sequence containing model and thinking changes produces matching
  ordered session records and JSONL without recompilation.
- Message parent links and existing `terminate` metadata remain unchanged.
- Reset clears records through the session actor mailbox.
- `just ci` and the session replay suite pass without sleeps or direct state
  mutation.

## Explicitly separate

Compaction/branch navigation, custom entries, and operation-lane records remain
separate follow-up work until their Pi source semantics, storage lifecycle, and
actor events are mapped in detail.
