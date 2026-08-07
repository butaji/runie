# P60 — Pi session labels

Status: first increment implemented.

Pi Core represents labels as journal facts (`fact: "label"`) addressed by a
target entry. Runie now carries the same transition as
`AgentEvent::SessionLabelChanged`, reduces it through `SessionActor`, and
round-trips it through the session JSONL projection as a `label` record with
`targetId` and nullable `label`.

The TUI model deliberately treats this as a session-owned fact and does not
create a second mutable label store. A future parity increment can expose the
actor snapshot’s effective labels to declarative feed components once the
session tree projection needs them; replay and persistence semantics are the
source of truth first.

Validation: `cargo test -p runie-core session::tests --lib`.
