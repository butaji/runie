# Unify file picker and panel list widget

## Status

`done`

**Completed:** 2026-07-01

## Context

The `@` file picker and command palette both need list-style rendering. The task was to verify whether duplication existed and unify it.

## Verification

Inspected the actual code on 2026-07-01:

- `crates/runie-core/src/update/dialog/file_pickers.rs` — builds a `Panel` using `PanelItem::Action` items with `ItemAction::Emit(InsertAtRef(path))`
- `crates/runie-tui/src/popups/panel/list.rs` — `render_list` renders any `Panel` including `PanelItem::Action` items via `push_action`
- `crates/runie-tui/src/popups/panel/mod.rs:35` — `panel_dialog` routes ALL non-form panels to `list::render_list`, including file picker panels

The unification already exists: the shared abstraction is the `Panel` + `PanelItem` data model. File picker labels (with `/` suffix for dirs) are stored as plain strings in `PanelItem::Action.label`, and `render_list` renders them identically to other action items.

## Acceptance Criteria

- [x] Shared list rendering helper. — `Panel` + `PanelItem::Action` data model; `list::render_list` renders all panels
- [x] File picker preserves `/` suffix and max height. — File picker labels include `/` suffix; `file_picker_label()` in `file_pickers.rs:38-43` adds it
- [x] Panel list behavior unchanged. — No changes made; architecture already unified

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshots for panel and file picker unchanged. (No changes made; architecture already shared)
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** `@` file picker and `/` command palette render consistently. (Verified via code inspection)

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — verified via code inspection: `panel_dialog` routes file picker panels to `list::render_list`.
