# P62 — Pi lane-aware entry append

Status: first actor/persistence increment implemented; YAML event bridge pending.

Pi Core’s `SessionStorage.appendEntry(entry, lane)` is a single state
mutation: it validates that the lane exists, sets the entry’s `parentId` to
that lane’s current leaf, appends the entry with the next sequence, and moves
the lane pointer to the new entry. Its JSONL entry carries the lane identity.

Runie’s current `SessionActor::append` is intentionally main-lane-only:
`SessionEntry` has no lane field, the reducer reads/writes one global
`leaf_id`, and JSONL import/export requires `lane: "main"`. The recently added
`SessionLaneChanged` facts therefore model lane pointers correctly, but cannot
yet append messages to a non-main lane without losing identity on persistence.

Required implementation:

- add lane identity to the actor-owned message entry projection and JSONL
  codec;
- introduce an event/mailbox append-to-lane operation with Pi validation;
- atomically update the selected lane leaf in the same reducer step;
- preserve branch queries, labels, name facts, fork mutations, and operation
  records across multiple lanes;
- add YAML replay/state assertions for append, invalid lane, branch lookup,
  and JSONL round-trip.

Do not emulate this by changing the global `leaf_id` or by inferring lane from
operation records; that would violate both Pi semantics and the SSOT actor
boundary.

Current increment: `SessionSnapshot::entry_lanes` is an actor-owned lane
identity projection; `SessionActor::append_to_lane` validates and appends with
the selected lane leaf, updates only that lane, and preserves identity in
JSONL. Focused regressions cover parent selection, invalid lanes, and import.
The remaining bridge is a typed application event plus YAML `session_append`
syntax so this path is also replayable without compiled fixture code.
