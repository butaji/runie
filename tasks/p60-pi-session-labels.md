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
create a second mutable label store. YAML scenarios can assert
`state.session_labels` without fixture-specific Rust code. The same event/fact
seam now carries Pi session names and exposes `state.session_name` for replay
assertions.

Validation: `cargo test -p runie-core session::tests --lib`.
