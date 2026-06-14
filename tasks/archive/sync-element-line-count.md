# Remove Manual Element Line-Count Sync Between Core and TUI

**Status**: done
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

## Resolution

The line count for an element is now derived from the same rendering logic that produces the actual lines.
The core exposes `element_line_count` via the renderer's `to_lines`, eliminating the need to keep two functions in sync.

Files: `runie-core/src/ui/elements.rs`, `runie-core/src/ui/transform.rs`, `runie-core/src/snapshot.rs`, `runie-tui/src/ui.rs`, `runie-core/src/tests/scroll.rs`, `runie-core/src/tests/vim_element_jump.rs`.

Archived in tasks/archive/.
