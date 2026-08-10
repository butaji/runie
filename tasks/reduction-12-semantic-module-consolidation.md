# Reduction 12: semantic module consolidation

Status: partial

After behavior stabilizes, consolidate numbered/generated fragments into
semantic modules and remove obsolete indirection.

Acceptance: source inventory remains valid, public APIs stay stable, and lint
plus workspace tests pass.

Current gap: numbered feed fragments and several oversized semantic modules
remain; consolidation is deliberately deferred until the feed model settles.

Progress: event projection tests now live in `events_tests.rs`, keeping the
production projection module below the structural file-size limit.
Palette matching helpers now live in `ui_palette.rs`, reducing the UI module's
size without changing its public API.
Dialog tests now live in `dialog_tests.rs`, and static provider/theme data was
removed from oversized functions. The remaining numbered feed fragments are
still intentionally retained while normalized feed state is incomplete.
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
