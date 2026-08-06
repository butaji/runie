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
- Added `ComponentSpec`/`StateOwner` metadata and the `CHAT_COMPONENTS`
  registry. Each declarative region now names its semantic component and the
  sole actor that owns its state.
- Added terminal-independent `PaintIntent` values. Opaline/Ratatui style
  resolution is now explicitly an adapter concern in `appearance.rs`.
- Added immutable `ChatViewProps` and reactive overlay slots for welcome,
  shortcuts, command palette, and doctor hint states. The tree is now a pure
  function of view props rather than a fixed terminal layout list.
- Added the thin `view!` macro for vertical/horizontal composition and slots;
  it expands directly to `Element` constructors and has expansion-level tests.
- Connected the live `App` path through `view_tree()`: actor snapshots now
  produce one immutable `ChatViewProps` projection, and the terminal binary
  uses its overlay slots for doctor, shortcuts, and command-palette decisions.
- Added `HeaderViewProps` and routed live header meter/theme acquisition
  through the same App projection boundary.
- Added pure `LayoutNode`, `LayoutEntry`, `StackLayout`, and `ScrollLayout`
  contracts plus actor-projection `ScrollState`. The scroll reducer models
  pi's follow-end, clamping, viewport growth, and user-scroll handoff without
  depending on Ratatui or terminal cells; focused tests pin the transitions.
- Added pure stack allocation with intrinsic sizes, fixed/auto basis, grow,
  shrink, minimum, maximum, and gap handling. The resolver is covered by a
  renderer-independent allocation test; integrating it with the live layout
  adapter remains the next boundary.
- Integrated the resolver into the live chat layout for the header,
  scrollback, and prompt regions. The existing Grok prompt/status gap and
  footer geometry remain explicit adapter constraints, and the full visual
  suite remains unchanged.
- Added YAML `visual.layout` region assertions and covered the 40×12 resize
  scenario. The runner checks header, scrollback, prompt, status, and footer
  coordinates through the live layout adapter before screen-text assertions.
- Added `visual.layout_matrix` and declared the `Hey` feed geometry contract
  at 62×32, 80×24, 100×30, and 120×36. The YAML runner re-renders each
  matrix case and validates all five regions without recompiling fixtures.
- Added renderer-independent `TuiSnapshot` in `runie-tui-model`, aggregating
  UI, feed, prompt, and status actor projections for one immutable view pass;
  `App::view_tree()` now consumes this aggregate.

## Next boundaries

1. Represent component props from actor snapshots as immutable view models.
2. Add stack measurement/reflow from `LayoutEntry` basis/grow/shrink values.
3. Add a terminal-independent cell/style intent layer.
4. Adapt scrollback, prompt, status, and overlays one component at a time;
   preserve existing YAML replay snapshots throughout migration.

The DSL is intentionally small: it describes structure and ownership without
hiding behavior in macros or introducing a second mutable state store.
