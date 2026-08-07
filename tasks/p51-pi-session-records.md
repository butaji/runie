# p51 — Pi session configuration records

Status: in progress — all three configuration records and JSONL round-trip
implemented; broader Pi record families remain open (2026-08-07)

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

1. Add a renderer-independent configuration record projection. The first
   increment adds model and thinking records without fabricating messages.
2. Preserve one ordered sequence/parent/leaf invariant across all records;
   message-only `entries` remains a compatibility projection until callers are
   migrated.
3. Add explicit actor mailbox commands generated from `AgentEvent::ModelChanged`
   and `ThinkingLevelChanged`; active tool changes need a typed core event before
   the session actor can reduce them.
4. Extend validated JSONL v4 import/export for these record types.
5. Add runtime YAML assertions for ordered record kinds and payloads; no
   fixture-specific Rust scenario code.

## Acceptance

- A YAML event sequence containing model and thinking changes produces matching
  ordered session records and JSONL without recompilation.
- Message parent links and existing `terminate` metadata remain unchanged.
- Reset clears records through the session actor mailbox.
- `just ci` and the session replay suite pass without sleeps or direct state
  mutation.

## First implementation increment (2026-08-07)

`SessionActor` now reduces `ModelChanged` and `ThinkingLevelChanged` through
its owned mailbox into ordered `SessionSnapshot::config_records`. Validated
JSONL v4 import/export preserves their `type`, payload, sequence, parent, and
timestamp metadata. The YAML `visual-status-working.yaml` fixture asserts the
ordered kind after the same event sequence that drives the TUI status.

`active_tools_change` now has a typed application event, actor reduction, YAML
replay coverage, and JSONL round-trip coverage. Compaction and operation-lane
records remain separate follow-up work.

## Branch-summary boundary audit (2026-08-07)

Pi's `BranchSummaryEntry` carries `fromId`, `summary`, optional `details`, and
the normal entry base. The source reducer couples it to a `navigation` intent
(`targetId`, `summarize`, optional `summaryEntryId`) and uses the summary entry
when reconstructing the selected branch context. The implementation now
introduces `BranchSummaryCreated` with navigation identity and reduces it into
the same actor-owned ordered journal. Full branch navigation/context
reconstruction remains separate; this slice does not pretend that a summary
record alone implements navigation.

Custom extension records now also use `CustomSessionEntryCreated` and preserve
`customType`/`data` through the actor journal and JSONL. YAML replay covers the
payload path without compiling extension-specific test code.

Compaction payloads now use `CompactionCreated` and preserve summary, retained
tail, token count, optional details, and usage through the same event, actor,
JSONL, and YAML paths. This is journal parity only; the compaction algorithm,
context trimming, and operation lifecycle remain outside this slice.

## Explicitly separate

Full branch navigation and operation-lane records
remain separate follow-up work until their Pi source semantics, storage
lifecycle, and actor events are mapped in detail.

The generic operation-lane increment now preserves declared operation record
kinds and payloads through `OperationRecordCreated`, the actor journal, JSONL,
and YAML replay. It is deliberately lossless transport, not an implementation
of Pi's operation admission/reducer policy; those semantics remain the next
operation-lane task.

`visual-operation-lifecycle.yaml` now exercises start → abort → finish and
asserts the final empty `active_operations` projection entirely at runtime.

## Navigation intent projection (2026-08-07)

Pi's `operation_started` record can carry a `navigation` intent with
`targetId`, `summarize`, and an optional `summaryEntryId`. `SessionActor` now
reduces that intent into the actor-owned `SessionSnapshot::navigation` field;
the JSONL importer reconstructs the same projection from the lossless
operation record. `visual-navigation-intent.yaml` exercises and asserts the
projection without compiled scenario code. This is intentionally the intent
projection only: Pi's full branch tree/context reconstruction, target
validation, and navigation admission/outcome policy remain open.
