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

## Completed slice (2026-08-07, actor-owned preparation)

`SessionActor::prepare_compaction` now routes deterministic token estimates and
the retention budget through its mailbox and returns the pure preparation
result asynchronously. The actor remains the sole reader of live session
state; no caller mutates or races a snapshot. Summary generation and the
event-owned `CompactionCreated` journal mutation remain the next boundary.

## Completed slice (2026-08-07, atomic storage actor)

`SessionStorageActor` now stages serialized JSONL at a sibling `.tmp` path and
atomically renames it into place. A failed rename removes the staged file and
returns an error; serialization remains a pure `SessionSnapshot::to_jsonl`
operation before the storage mailbox receives the contents. The actor owns
the asynchronous filesystem task, and its test validates the v4 header and
absence of the temporary file after publication.

The same actor now owns loading: it reads the file, applies the pure Pi
torn-tail repair boundary, and returns validated `(session_id, cwd, snapshot)`
data through its mailbox. Caller code does not perform filesystem reads or
direct snapshot replacement.

`SessionStorageActor::fork_snapshot` now applies the validated pure fork
inside the storage mailbox and atomically publishes the re-sequenced fork to
a separate path. The round-trip test publishes, forks, reloads, and checks the
new leaf/sequence boundary.

Storage integration coverage now appends an invalid final physical line and
loads it through the actor, proving the Pi torn-tail rule at the filesystem
boundary rather than only in a pure parser test.

The operation-lane JSONL round-trip now asserts that admitted record families
survive export/import in order, in addition to lifecycle projections.

## Completed slice (2026-08-07, lane event DSL)

`session_lane_event!` and `session_lane_record_name!` provide explicit macro
arms for all nine Pi operation-lane families. Rust adapters now get a typed
family choice while retaining Pi's JSON payload shape; unknown strings cannot
be created through this DSL. A test constructs every family and verifies the
wire names.

## Completed slice (2026-08-07, typed internal lane boundary)

`SessionLaneRecord` now decodes the nine Pi operation-lane families into a
closed internal enum before admission and reduction. Each variant retains the
lossless JSON payload, so forward-compatible Pi fields still survive JSONL
round trips while Rust code cannot silently branch on an unknown family. The
reducer uses this typed value for lifecycle matching, and unit coverage checks
both family identity and payload preservation.

## Completed slice (2026-08-07, YAML all-family replay)

`visual-operation-lane-families.yaml` now drives all nine Pi lane families
through the runtime event DSL. The YAML oracle verifies ordered retention and
the terminal operation projection, so changing this scenario does not require
recompiling Rust tests. The fixture is included in automatic discovery and
passes the complete YAML replay suite.

## Completed slice (2026-08-07, live tool-start emission)

The session bus bridge now projects the authoritative `ToolExecutionStart`
event into a `tool_started` lane fact through the `SessionActor` mailbox. The
record keeps the tool call identity, name, and arguments losslessly in the Pi
JSON payload; no tool or renderer code mutates session state. A session actor
test verifies the event arrives before the terminating tool result, and the
existing YAML all-family fixture remains the replay oracle for the same lane
family.

## Current Runie mapping

`runie-core/src/session.rs` owns parent-linked message/config entries and
reduces generic `OperationRecordCreated` facts into active operation,
outcome, kind, error, and navigation projections. JSONL v4 export/import and
terminated message metadata are covered by replay tests.

### Pi compaction contract audit (2026-08-07)

The upstream source audit covers both `packages/agent/src/harness/compaction`
and `packages/agent/src/harness/session/context.ts`. A complete compaction is
not only a cut-point calculation:

- `defaultContextEntryTransform` keeps the newest `compaction` entry and the
  path after it, dropping the compacted prefix from the next provider context;
- `sessionEntryToContextMessages` materializes the compaction summary message
  followed by `retainedTail`, skips deferred assistant messages, and projects
  branch summaries and registered custom entries;
- the generated result persists `summary`, `tokensBefore`, `retainedTail`,
  optional model `usage`, and implementation `details` (file read/modified
  sets in the built-in compactor);
- the operation lane separately records admission, step attempt, result
  entry, and completion outcome for a `compaction` operation kind, including
  retry/abort/failure distinctions.

Runie initially had only the pure index partition and lossless
`CompactionCreated` journal payload. The typed context-message increment below
now applies the latest compaction boundary and represents its summary before
provider conversion; live summarization and compaction-operation admission
remain separate gaps.

### Event/schema boundary audit (2026-08-08)

The upstream context projector calls `createCompactionSummaryMessage`, which
emits a distinct `role: "compactionSummary"` message carrying `summary`,
`tokensBefore`, and `timestamp`, followed by the retained tail. The existing
`CompactionCreated` event must not be coerced into a user message or
constructed by a provider/TUI caller.

The typed message variant is now present and reduced from the actor-owned
projection. It carries the persisted timestamp and preserves ordering with
the retained tail. YAML replay drives `CompactionCreated` through the actor
mailbox and asserts the resulting context role sequence; no caller mutates a
session snapshot or appends a synthetic message directly.

This audit also confirms the general state-transfer rule: every state change
at this boundary is delivered as a typed event/message to the owning actor;
renderers and provider adapters may only consume immutable snapshots or
context projections.

### Completed typed context-message increment (2026-08-08)

`CompactionSummaryMessage` is now an explicit `AgentMessage` variant. The
session projection materializes the ordered internal context sequence as
`compactionSummary`, retained-tail messages, and post-boundary messages. The
default provider conversion maps only the summary variant to Pi's user wire
message with the exact `COMPACTION_SUMMARY_PREFIX`/suffix and preserves its
timestamp. The TUI treats the internal message as non-feed context data.

The YAML `visual-status-working.yaml` replay asserts the internal role through
the session actor projection, while core tests assert serialization and the
provider conversion separately. The remaining compaction gap is narrowed to
Pi's live summarization owner and publication timing; no direct snapshot
mutation or provider-side synthetic message is used.

### Completed pure threshold increment (2026-08-08)

`runie_core::session::should_compact` now mirrors Pi's strict automatic
threshold decision: disabled settings never compact, otherwise compaction is
requested only when `contextTokens > contextWindow - reserveTokens`. The
subtraction is saturating for malformed oversized reserves, while the strict
comparison remains unchanged. YAML state assertions expose the decision with
runtime-declared context tokens, reserve, enabled flag, and expected result;
the working-state fixture exercises the over-threshold path. Boundary tests
cover equality, disabled settings, and oversized reserves.

### Completed context-usage increment (2026-08-08)

Runie now exposes pure `estimate_message_tokens` and
`estimate_context_tokens` functions matching Pi's conservative
four-characters-per-token heuristic, fixed image estimate, latest valid
assistant usage preference, and trailing-message accounting. Aborted/error
assistant usage is ignored; `Usage.totalTokens` is preferred and the component
sum is the fallback. Core tests cover latest-usage selection, tail estimates,
and invalid terminal usage. The summarization actor still owns when this
estimate is requested and published as an event; these functions do not mutate
session or provider state.

### Completed slice (2026-08-07, compaction context boundary)

`SessionSnapshot::compaction_context_projection` is now a pure projection of
the newest compaction record. It returns the persisted summary metadata and
retained tail, selects only message entries after the compaction sequence, and
filters deferred assistant results exactly at the context boundary. A core
regression constructs a parent-linked journal with a deferred post-boundary
entry, while the YAML state DSL asserts the selected entry IDs through the
real session actor path. No provider or TUI code mutates the session snapshot.

The following are not yet exact Pi parity:

- the public wire boundary still carries operation records as generic
  `(record_type, data)` values; the macro constrains Rust call sites, but the
  persisted representation is not a typed Rust record union;
- queue, deferred-write, tool-start, step-attempt, and usage records are
  admitted and replayed when supplied as events, but the live Pi adapter does
  not yet emit each family from its corresponding operation transition;
- compaction is journaled when supplied by an event, and deterministic cut
  preparation is actor-owned, but Runie does not yet implement Pi's complete
  context-building, summarization, and `CompactionCreated` result publication
  boundary.

### Queue emission audit (2026-08-07)

Pi's `queue_enqueued` record is not just a notification: it carries the queue
kind (`steer`, `followUp`, or `nextRun`), the owning `runId` where applicable,
and a provisioned target entry with a stable entry identity. `queue_cancelled`
then refers to that entry identity. Runie's queue actors currently own only a
`Vec<AgentMessage>` and expose push/drain operations; they do not yet allocate
or publish provisioned entry identities. Emitting a record from `LoopActor`
with a timestamp- or message-derived ID would not be Pi parity and would make
replay ambiguous. The required next boundary is therefore queue-actor-owned
identity allocation plus an event capability that publishes the exact record
after the mailbox accepts the message.

The first half of that boundary is now implemented: `SteeringQueueActor` and
`FollowUpQueueActor` allocate monotonic actor-owned identities (`steer-N` and
`follow-up-N`) when their push mailbox commands commit. The public push result
returns the acknowledged identity, while drain behavior remains unchanged for
existing consumers. Queue event publication and cancellation records remain
next, because they need the queue kind and active run context at the loop
boundary.

Queue publication is now wired at `LoopActor::steer`, `follow_up`, and the
corresponding clear methods. After an acknowledged queue mutation, the loop
publishes `OperationRecordCreated` facts with `queue_enqueued` or
`queue_cancelled`, including the actor-issued identity and serialized target.
The queue actors remain the sole owners of identity allocation; session
reduction still occurs through the event bus and `SessionActor`.

`visual-operation-queue-lifecycle.yaml` now provides the no-recompile replay
oracle for both `steer` and `followUp` enqueue/cancel pairs, asserting stable
identity-bearing records in order.

## Completed slice (2026-08-07, assistant usage emission)

When the session actor receives the existing `MessageEnd` event for an
assistant message, it now appends the message entry and derives a lossless Pi
`usage` lane record from that assistant's usage payload in the same mailbox
reduction. The generated `entryId` is the actor-issued message entry ID, so
usage identity cannot drift from the journal. Coverage verifies the event
sequence and identity; the all-family YAML fixture continues to exercise the
runtime usage record shape.

### Transition audit (2026-08-08)

Pi emits `step_attempt` before the assistant result is committed, with the
active operation ID, attempt number, step kind, and eventual result entry ID.
Runie's session mailbox now reserves the actor-issued result ID and emits this
fact before committing the assistant entry, so the former post-hoc ordering
defect is closed without a caller-owned identity.

Pi emits `write_deferred` when a deferred assistant result is persisted, with
the target provisioned entry and deferred handle. Runie has deferred provider
commands and handles, but no session mutation event at the persistence point.
The provider/loop boundary must publish that typed fact before this family can
be projected without inventing state.

The remaining limitation is operation correlation when multiple open lanes
exist: the reducer still selects its active-operation policy, and Pi-specific
admission/context selection must be mapped before concurrent operation kinds
can be expanded.

## Completed slice (2026-08-07, separated operation journal lane)

`SessionActor` no longer inserts `OperationRecordCreated` facts into the
parent-linked configuration entry lane. The mailbox reduces those facts
directly into the actor-owned ordered `lane_records` projection, preserving
message/config entry IDs and branch topology. This matches Pi's separate
message and operation lanes and makes live lifecycle publication safe for
existing message replay oracles.

## Completed slice (2026-08-07, actor-owned run lifecycle identity)

`LoopActor` now allocates monotonic run IDs inside its owning actor and passes
the identity into the async loop. The loop publishes `operation_started` after
the Pi agent-start event and `operation_finished` before agent-end, while YAML
exact Pi/UI traces intentionally exclude application-owned lane facts and
lane-specific assertions verify them separately.

## Completed slice (2026-08-07, deferred-write persistence event)

When `SessionActor` commits an assistant message whose stop reason is
`deferred`, the same mailbox reduction emits a `write_deferred` lane fact. Its
target and deferred handle are copied from the assistant message, and its
identity is the actor-issued journal entry ID. `deferred-response.yaml` now
asserts the complete lane sequence: operation start, usage, deferred write,
and operation finish.

## Completed slice (2026-08-07, correlated step attempt)

The session actor now correlates each committed assistant entry with the
active operation lane, counts prior attempts for that run, and emits a
`step_attempt` record containing `runId`, `step: assistant`, the monotonic
attempt number, and the real actor-assigned `resultEntryId`. Deferred writes
therefore retain the order `step_attempt`, `usage`, `write_deferred` within the
same reduction. YAML replay fixtures assert the live record ordering.

Lifecycle completion now derives the operation outcome from the actor-owned
abort signal, terminal state error, and final assistant stop reason. Aborted
assistant results therefore publish `operation_finished: aborted` instead of
being mislabeled as completed; `visual-aborted-turn.yaml` asserts this case.

## Completed slice (2026-08-07, separate-lane JSONL restart)

JSONL import now recognizes all typed operation-lane families before applying
message-lane sequence and parent-link validation. Restart therefore reduces
lane records directly, without creating configuration entries or changing the
message leaf/sequence topology. Export/import coverage explicitly asserts that
operation facts survive while `config_records` contains no operation records.

## Implementation order

1. Replace the generic Rust operation-record payload with a typed internal
   lane-record union while retaining lossless Pi JSONL compatibility.
2. Emit queue, deferred-write, tool-start, step-attempt, and usage records at
   their owning Pi operation transitions; each must reduce through
   `SessionActor` only.
3. Add the actor-owned compaction context/summarization result boundary and
   publish `CompactionCreated` through an event, with replay assertions for
   summary, retained tail, tokens, usage, and details.
4. Extend YAML state assertions to cover live emission and restart/recovery
   equivalence for every lane family.

## Acceptance

- every upstream lane record has an explicit event, reducer, and YAML trace;
- reloading a persisted actor snapshot produces the same ordered state;
- interrupted writes leave the prior published file valid and repair the
  final torn record deterministically;
- no TUI or provider code mutates session state directly;
- `just ci` and the source inventory remain green.

This workstream is required for the stated 100% Pi-core parity goal; it is not
classified as out of scope.
