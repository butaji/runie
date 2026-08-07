# P61 — Pi session lane projection

Status: implemented and fully replay-verified.

Pi Core’s `SessionMutation` union has a distinct `lane` mutation carrying
`seq`, `lane`, and `leafId`. It is not an operation-lane record and must not be
reconstructed from operation payloads. Pi validates that the lane exists (or
is being created) and that a non-null leaf points to an existing entry, then
records the mutation in the ordered log.

Runie currently owns message/configuration entries and operation-lane records
inside `SessionActor`, but `SessionSnapshot` has no actor-owned lane map or
lane-log projection. This is the next session parity gap. The implementation
must add:

- an application `SessionLaneChanged` event and actor mailbox reduction;
- immutable `SessionSnapshot::lanes` and ordered lane facts;
- validation before snapshot publication, including rejected mutation
  immutability;
- JSONL `kind: "lane"` import/export preserving sequence and leaf identity;
- YAML event and state assertions so lane/fork scenarios require no Rust
  fixture code;
- event-sequence tests for create, move, invalid target, and reset.

Implemented increment: lane facts now have an actor mailbox API, validation,
pure latest-leaf projection, and `kind: "lane"` JSONL round-trip. The focused
core regression covers create, persistence, and invalid-target immutability.
`session-lanes.yaml` now covers create/move-to-null/reset and asserts
`state.session_lanes` through the runtime replay harness. JSONL serialization
sorts message, configuration, lane, and operation records by their shared
sequence before writing; an interleaved lane/configuration regression pins
that ordering and round-trip behavior. `fork_at_message` now follows Pi’s
fork mutation contract by publishing a fresh `main` lane pointer after the
copied branch prefix; the branch regression asserts the new lane head and
sequence. Fork metadata now also re-emits the effective session name and
labels for retained entries, matching Pi’s `createForkMutations` behavior
even when those facts were recorded after the branch point.

Admission now distinguishes `create` from `move`: duplicate creation and
moving a missing lane are rejected inside the actor before snapshot publication.
The YAML lane event exposes this distinction declaratively.

The lane projection must remain separate from `operation_kinds` and
`active_operations`; those are Pi operation records, not session tree facts.
