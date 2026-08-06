# p26 — Declarative TUI view DSL

## Objective

Separate the actor-owned model and declarative component tree from terminal
layout and Ratatui rendering details, following the React/HTML/browser split:
view functions describe elements; the layout adapter resolves geometry; the
renderer paints terminal cells.

## Progress

- Added `crates/runie-tui/src/view.rs` with pure `Element`, `Slot`, and
  `Direction` nodes plus the canonical chat region tree.
- Added `layout::chat_elements()` as the adapter seam. Existing geometry is
  unchanged and remains the terminal-specific layer.
- Added a structural unit test proving stable region ordering.

## Next boundaries

1. Represent component props from actor snapshots as immutable view models.
2. Add a terminal-independent cell/style intent layer.
3. Adapt scrollback, prompt, status, and overlays one component at a time;
   preserve existing YAML replay snapshots throughout migration.

The DSL is intentionally small: it describes structure and ownership without
hiding behavior in macros or introducing a second mutable state store.
