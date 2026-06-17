# Dedupe Small Helpers

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Several small helpers are duplicated across the workspace: `now()` (message.rs + update/mod.rs), `which_tool` (tool/grep.rs + tool/find.rs + agent/tools.rs), and `display_width` (core vs tui status_bar.rs).

## Acceptance Criteria

- [ ] A single `now()` helper exists (e.g. `runie_core::time::now`).
- [ ] A single `which_tool` helper exists (e.g. `runie_core::tool::which_tool`).
- [ ] `runie-tui::status_bar` uses `runie_core::display_width::width`.
- [ ] Duplicates are deleted.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `now_helper_unique` — only one `now()` definition compiles.
- [ ] `which_tool_unique` — only one `which_tool` definition compiles.
- [ ] `status_bar_width_uses_unicode` — wide characters are counted correctly.

## Files touched

- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/tool/grep.rs`
- `crates/runie-core/src/tool/find.rs`
- `crates/runie-agent/src/tools.rs`
- `crates/runie-tui/src/status_bar.rs`

## Notes

None.
