# Pi session lane and durable storage parity

Status: in_progress

## Completed slice (2026-08-07)

`runie-core` now classifies all nine Pi operation-lane record families with
`SessionLaneRecordKind`. A pure validator checks the known record type and
record identity (`id`, `runId`, or `entryId`) and rejects duplicate open
operations before they can change the actor-owned operation projection.
Malformed or duplicate generic wire events remain journal facts for forward
compatibility, but the reducer ignores them rather than corrupting state.
Unit tests cover family classification, usage identity, malformed records,
duplicate admission, and the existing lifecycle replay.
The runtime YAML fixture `visual-operation-admission.yaml` covers the same
event sequence through the SessionActor and asserts that only the valid start
and finish alter the projection.

This is intentionally only the first admission boundary. Typed event
variants, sequence/lane validation, durable storage, and recovery remain open.

## Completed slice (2026-08-07, JSONL recovery)

`SessionSnapshot::repair_jsonl_torn_tail` is now the pure recovery boundary for
the JSONL loader. It normalizes a valid final line, discards only an invalid
final physical line (the Pi torn-tail rule), and rejects invalid non-final
lines. `SessionActor::restore_jsonl` uses this boundary before validated
import. Unit tests cover both recoverable final corruption and unrecoverable
middle-of-file corruption.

The validator also accepts legacy pre-storage events while validating the
complete Pi storage tuple when present: non-empty `lane`, positive `seq`, and
non-negative `timestamp`. The YAML admission fixture exercises that metadata
path.

## Completed slice (2026-08-07, ordered lane projection)

`SessionSnapshot` now retains every admitted operation-lane event in
`lane_records`, including its type, identity, storage metadata, and lossless
JSON payload. Invalid and duplicate records are excluded before projection.
The YAML state oracle supports `session_lane_records`, and the admission
fixture proves that only the valid start/finish sequence is retained.

## Completed slice (2026-08-07, pure fork prefix)

`SessionSnapshot::fork_at_message` now validates a message target, copies only
its selected branch prefix, re-sequences the forked journal from one, and
rebuilds admitted operation projections through the same reducer. The source
snapshot is not mutated; invalid targets return an error. This is the pure
core needed by the future actor-owned atomic fork writer.

## Source contract

The authoritative upstream files are:

- `~/Code/agents/pi/packages/agent/src/harness/session/types.ts`
- `~/Code/agents/pi/packages/agent/src/harness/session/jsonl/storage.ts`
- `~/Code/agents/pi/packages/agent/src/harness/session/jsonl/codec.ts`
- `~/Code/agents/pi/packages/agent/src/harness/compaction/compaction.ts`

Pi's session model has two ordered lanes. The message lane contains parent-
linked entries; the operation lane contains typed records. The covered record
families are `operation_started`, `abort_requested`, `operation_finished`,
`step_attempt`, `tool_started`, `queue_enqueued`, `queue_cancelled`,
`write_deferred`, and `usage`. Records carry an id, sequence, lane, and
timestamp, and operation admission rejects a second open operation in one
lane.

The JSONL backend creates and loads a versioned header, appends mutations in
sequence order, repairs an unterminated/torn final line, and publishes a
complete temporary file atomically before rename. Forks copy a validated
branch prefix into a new session. Compaction entries preserve the summary,
retained tail, token count, usage, and implementation details.

The compaction audit identifies the pure algorithm still required: Pi estimates
context tokens from the latest valid assistant usage plus trailing messages,
applies `contextWindow - reserveTokens` as the threshold, finds valid cut
points at message/branch-summary boundaries, preserves a recent-token budget,
and separates a split-turn prefix from the retained tail. `prepareCompaction`
then carries `previousSummary`, `tokensBefore`, retained messages, and
file-operation details into the async summarization boundary. Runie currently
journals `CompactionCreated` but does not yet implement this preparation
algorithm.

## Completed slice (2026-08-07, pure cut-point planner)

`find_compaction_cut_point` now provides the first pure compaction boundary.
It accepts caller-owned token estimates, excludes tool-result entries from cut
points, preserves the recent-token budget, and reports a split-turn prefix for
the async summarization owner. Unit coverage pins the split-turn behavior.
The YAML runner now accepts runtime token estimates and cut-point assertions;
`hello-streaming.yaml` exercises the contract without recompilation.

The same fixture now asserts the full pure preparation partition: history,
split-turn prefix, retained tail, and `tokens_before`. Async summarization and
event publication remain separate because they require an owned actor.

## Current Runie mapping

`runie-core/src/session.rs` owns parent-linked message/config entries and
reduces generic `OperationRecordCreated` facts into active operation,
outcome, kind, error, and navigation projections. JSONL v4 export/import and
terminated message metadata are covered by replay tests.

The following are not yet exact Pi parity:

- operation records are generic `(record_type, data)` rather than typed lane
  records with admission and sequence validation;
- there is no durable actor-owned JSONL storage backend, atomic publish, torn
  tail repair, or fork writer;
- queue, deferred-write, tool-start, step-attempt, and usage records are not
  emitted as their own session events;
- compaction is journaled when supplied by an event, but Runie does not yet
  implement Pi's context-building and summary/retained-tail algorithm.

## Implementation order

1. Add a typed `SessionLaneRecord` event DSL while retaining the Pi wire event
   boundary; each variant must reduce through `SessionActor` only.
2. Add pure record admission/sequence/parent validation and YAML fixtures for
   every lane family, including duplicate-open-operation rejection.
3. Add an async owned JSONL storage actor with atomic temp-file publication,
   torn-tail repair, load/export, and fork operations.
4. Add compaction context/result events and replay assertions for summary,
   retained tail, tokens, usage, and details.
5. Add YAML state assertions for ordered records and restart/recovery state;
   only then promote the session inventory row to covered.

## Acceptance

- every upstream lane record has an explicit event, reducer, and YAML trace;
- reloading a persisted actor snapshot produces the same ordered state;
- interrupted writes leave the prior published file valid and repair the
  final torn record deterministically;
- no TUI or provider code mutates session state directly;
- `just ci` and the source inventory remain green.

This workstream is required for the stated 100% Pi-core parity goal; it is not
classified as out of scope.
