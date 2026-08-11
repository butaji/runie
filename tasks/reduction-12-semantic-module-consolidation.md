# Reduction 12: semantic module consolidation

Status: partial

After behavior stabilizes, consolidate numbered/generated fragments into
semantic modules and remove obsolete indirection.

Acceptance: source inventory remains valid, public APIs stay stable, and lint
plus workspace tests pass.

Current gap: several oversized semantic modules remain; feed fragment
consolidation is complete now that the normalized feed model has settled.

Progress: event projection tests now live in `events_tests.rs`, keeping the
production projection module below the structural file-size limit.
Palette matching helpers now live in `ui_palette.rs`, reducing the UI module's
size without changing its public API.
Dialog tests now live in `dialog_tests.rs`, and static provider/theme data was
removed from oversized functions. Remaining semantic modules are retained
until their behavior-specific boundaries can move without obscuring ownership.
Structural lint now reports no feed-owned issues; remaining findings are in
command/palette/UI modules owned by the parallel command-surface work.
The command-surface ownership boundary is now clean as well: command tests
were moved to `commands_tests.rs`, UI messages to `ui_messages.rs`, and the
extended app command route to `app_extended_command.inc`. `lint-check` is now
clean across the workspace.
Tool-row lifecycle and transcript-selection fragments are now consolidated in
the semantic `feed_state_tool_rows.rs` module, with the same private methods
and include boundary.
Tool start/finish and display-mode fragments are consolidated in
`feed_tool_lifecycle.rs`; mode settlement now uses one key fold instead of
repeating the two map writes.
Activity facts and transcript presentation transitions are consolidated in
`feed_activity.rs`, keeping activity ownership and spacing behavior together.
Workflow start/progress/end and transcript replacement are consolidated in
`feed_workflow.rs`, preserving the same lifecycle transitions.
Transcript and dense-tool navigation are consolidated in `feed_navigation.rs`,
keeping selection transitions and group identity in one semantic module.
Tool display-mode transitions and identity projection are consolidated in
`feed_tool_display.rs`.

Provider effort mapping now lives in semantic `types_thinking.rs`, the
provider effort matrix has its own fixture module, and MCP HTTP registration
is split into discovery/request/call-hook helpers. Older actor, MCP, and feed
modules remain queued for consolidation.
The persistent stdio MCP actor now delegates its mailbox lifecycle to an
owned worker function and a small call reducer, keeping construction separate
from async transport state transitions.
Logical range and terminal-cell selection transitions are consolidated in
`feed_selection.rs`.
Local transcript line mutations are consolidated in `feed_line_ops.rs`.
Assistant-turn settlement is consolidated in `feed_assistant.rs`.
Obsolete legacy palette metadata declarations were removed from the semantic
palette module after call-site audit, reducing dead indirection without
changing its public API.

Background job mailbox reduction now delegates start/cancel/cancel-all events
to small owned handlers while `run_worker` retains the async completion select;
the actor lifecycle remains unchanged.
Tool-card summary facts now live with the semantic feed tail projection rather
than the large tool-type fragment, reducing the structural module without
changing the public feed API.
MCP stdio close handling is now a separate owned transition helper, keeping
the worker loop below the structural function threshold.
Replay terminal-marker reduction and compaction threshold assertions now use
small semantic helpers, keeping event/state tests focused on data tables and
boundaries.
Tool executor worker settlement now separates scheduler event reduction from
outcome projection, keeping cancellation/success semantics in typed helpers.
Actor tests now live in `actor_tests.rs`, preserving the event/replay coverage
while keeping the production actor module focused on mailbox ownership and
state transitions. MCP stdio request writing, response correlation, and tests
are likewise separated into small semantic helpers/modules; the remaining
MCP module size is still queued for a transport/domain split.
Interactive question and web-search executor adapters now live in
`executor_special.rs`, leaving the main dispatcher focused on scheduling and
tool lifecycle transitions.
The MCP transport/domain split is now complete: HTTP request/response policy
lives in `mcp_http_transport.rs`, persistent stdio protocol state lives in
`mcp_stdio_transport.rs`, and the root module retains shared lifecycle and
tool data. Structural lint is clean again.
