# Reduction 03: declarative scrollback events

Status: partial

Replace layered `ScrollbackMsg` routing with grouped typed events and smaller
data operations without changing transcript behavior.

Progress: reducer-stage routing now uses a local declarative macro, and
`ScrollbackMsg::domain()` is generated from one grouped domain table. This
removes repeated forwarding and predicate boilerplate while preserving the
explicit lifecycle, content, tool, workflow, and navigation behavior. The
event vocabulary itself remains compatibility-oriented rather than fully
grouped into typed payload enums.

Acceptance: existing feed event-sequence tests remain green and new grouped
event tests cover lifecycle, content, tool, workflow, and navigation paths.
