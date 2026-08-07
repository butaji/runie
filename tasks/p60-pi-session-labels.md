# P60 — Pi session labels

Status: implemented and replay-assertable.

Pi Core represents labels as journal facts (`fact: "label"`) addressed by a
target entry. Runie now carries the same transition as
`AgentEvent::SessionLabelChanged`, reduces it through `SessionActor`, and
round-trips it through the session JSONL projection as a `label` record with
`targetId` and nullable `label`. The actor rejects labels whose target is not
an existing message entry, and `SessionSnapshot::labels()` is the pure
effective-state projection, including removals.

The TUI model deliberately treats this as a session-owned fact and does not
create a second mutable label store. A future parity increment can expose the
actor snapshot’s effective labels to declarative feed components once the
session tree projection needs them; replay and persistence semantics are the
source of truth first. YAML scenarios can assert `state.session_labels`
without fixture-specific Rust code.

Validation: `cargo test -p runie-core session::tests --lib`.
