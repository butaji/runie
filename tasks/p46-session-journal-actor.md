# p46 — Actor-owned Pi session journal

Status: implemented (2026-08-06)

Runie now has a minimal session-tree foundation in `runie-core::session`:

- `SessionActor` is the sole owner of ordered message entries and the current
  leaf pointer.
- `MessageEnd` events append entries with sequence, parent, timestamp, and
  message payload; `Reset` clears the journal through the same bus boundary.
- `SessionSnapshot` is immutable and can be consumed by persistence or TUI
  projections without sharing mutable state.
- `flush` gives replay/integration code an acknowledgement boundary without
  timers or sleeps.
- `hello-streaming.yaml` asserts the resulting entry count through the real
  loop, bus, and actor path at runtime.
- `App::new` and `App::new_with_welcome` now attach the same actor-owned
  journal to the live event bus, and expose snapshot/flush accessors without
  making rendering responsible for session state.
- `SessionSnapshot::to_jsonl` emits a pure Pi JSONL v4 header plus parent-linked
  message entries; filesystem writes remain a separate storage boundary.
- `SessionSnapshot::from_jsonl` is the inverse pure boundary for this supported
  message lane. It validates the v4 header, required fields, consecutive
  sequence numbers, and parent links, and rejects unsupported entry kinds
  instead of silently importing partial state. Export/import round-trip and
  malformed-input tests exercise these invariants.

This is intentionally the journal seam, not a claim that Pi JSONL storage,
forking, compaction, labels, or durable filesystem recovery are complete.
Those follow-up contracts must build on this actor and preserve event ordering.
