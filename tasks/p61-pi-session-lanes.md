# P61 — Pi session lane projection

Status: source-audited; implementation next.

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

The lane projection must remain separate from `operation_kinds` and
`active_operations`; those are Pi operation records, not session tree facts.
