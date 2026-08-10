# Reduction 03: declarative scrollback events

Status: partial

Replace layered `ScrollbackMsg` routing with grouped typed events and smaller
data operations without changing transcript behavior.

Progress: reducer-stage routing now uses a local declarative macro, and
`ScrollbackMsg::domain()` is generated from one grouped domain table. Grouped
typed lifecycle, tool, and workflow events now bridge to the compatibility
message vocabulary through one pure conversion boundary. This removes repeated
forwarding and predicate boilerplate while preserving existing consumers.
`ScrollbackActor::apply_grouped` now accepts those producer-intent events at
the owned mailbox boundary; lifecycle, content, and navigation groups are
covered by actor tests.

Acceptance: existing feed event-sequence tests remain green and new grouped
event tests cover lifecycle, content, tool, workflow, and navigation paths.
Content, tool, and workflow compatibility routing now share the semantic
`feed_reducers.rs` module while retaining the same typed reducer boundaries.
The ordered reducer stage machine and navigation fallback are consolidated in
`feed_reducer_boundary.rs`.
