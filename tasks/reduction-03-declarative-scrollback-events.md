# Reduction 03: declarative scrollback events

Status: adopted

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
The grouped lifecycle event vocabulary now uses one typed macro table to
generate its serde enum and compatibility-message mapping, keeping producer
intent and the legacy bridge in one inspectable declaration.
Transcript line variants and their prefix projection now use the same
data-shaped declaration, keeping the feed vocabulary and terminal rail in
sync.

Completion evidence: grouped lifecycle, content, tool, workflow, and
navigation replay tests pass; `ScrollbackActor::apply_grouped` is exercised
through its owned mailbox; and the full workspace lint/test/replay gates plus
the live TUI smoke suite pass.
