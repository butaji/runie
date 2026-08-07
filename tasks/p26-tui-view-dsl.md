# p26 — Declarative TUI view DSL

## Semantic hint naming correction (2026-08-06)

The declarative component tree now names the Grok small-screen surface
`CompactModeHint` end-to-end. The previous `DoctorHint` identity was a stale
compatibility name left behind after the renderer was aligned to Grok's
`Tight on space? Try /compact-mode` behavior. Visibility remains an immutable
view prop derived from the actor snapshot; terminal-row gating remains a pure
layout predicate in the renderer adapter.

## Objective

Separate the actor-owned model and declarative component tree from terminal
layout and Ratatui rendering details, following the React/HTML/browser split:
view functions describe elements; the layout adapter resolves geometry; the
renderer paints terminal cells.

## Layer contract

1. **Model** — actor-owned reducers emit immutable `TuiSnapshot` values. The
   view layer never reads actor internals or mutates state.
2. **Declarative view** — pure functions produce `ChatViewProps`,
   `ViewDocument`, `Element`, ownership metadata, and semantic `PaintIntent`
   values. This answers *what exists*, without terminal dimensions, Ratatui
   buffers, cursor coordinates, or ANSI capability knowledge.
3. **Layout adapter** — pure geometry resolves slots against the viewport and
   intrinsic component measurements. Resize changes allocation, not semantic
   component identity.
4. **Renderer** — the only terminal-facing layer maps projections and paint
   intents to Ratatui cells, styles, cursor placement, and terminal effects.
   It is non-blocking and returns input/lifecycle changes as events to the
   owning actor.

This is Runie's React/HTML/browser-core analogue: `TuiSnapshot` is the data
model, `ViewDocument` the declarative document, layout the browser layout pass,
and Ratatui the terminal backend. New parity details must enter through this
boundary rather than through renderer-only state or direct actor reads.

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
- Added `layout_entries!` for mixed fixed/grow declarative stack entries; it
  expands directly to `LayoutEntry` constructors and is used by the canonical
  chat layout without hiding allocation behavior. A pure expansion test pins
  the macro contract.
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
- The live binary now acquires that aggregate once per draw before deriving
  overlay slots, avoiding an additional independent actor read for the same
  declarative tree. Ratatui widget compatibility reads remain isolated to
  terminal-specific painting and cursor geometry.

- Documented the four-layer contract above as the acceptance rule for future
  parity work: model, declarative view, layout adapter, then renderer.
- `PromptSnapshot` now owns pure emptiness and intrinsic-height calculations;
  App layout and live key/settled-feed decisions consume those model facts
  instead of asking the compatibility widget.
- `ViewDocument` is now preserved through the `App` projection boundary:
  `view_document_from_model()` retains both the element composition and the
  component/state-owner registry, while `view_tree()` remains a compatibility
  accessor for callers that only need the root. The live `App::render()` pass
  also takes one aggregate `TuiSnapshot` before deriving layout and feed
  rendering inputs, preventing an inconsistent frame from mixed actor reads.

- `ViewDocument` now carries immutable `ViewProps` for the chat overlays and
  header. The live binary consumes the header props from that same document;
  it no longer reconstructs header meter/theme facts from a compatibility
  widget during painting. `StatusSnapshot::header_meter()` keeps that
  projection renderer-independent.

- `ViewProps` now also carries the feed, prompt, status, and UI actor
  projections. Both live draw paths consume those values from the one
  document produced for the frame; the pre-draw readiness check remains an
  event-loop decision, outside painting.

### Boundary audit (2026-08-06)

Re-audited both live draw paths after the feed-model migration. The binary
acquires one `TuiSnapshot`, derives one immutable `ViewDocument`, and passes
only document props into the terminal adapter. Prompt intrinsic height comes
from `PromptSnapshot`; feed/status widgets are rehydrated from actor snapshots;
no renderer reads actor state or asks a compatibility widget to measure layout.
The remaining p26 items are broader declarative reflow and terminal-independent
style-intent coverage, not an active duplicate state or measurement path.

## Next boundaries

1. Replace remaining compatibility widget-derived overlay/layout measurements
   with explicit immutable component props where parity requires them.
2. Add stack measurement/reflow from `LayoutEntry` basis/grow/shrink values.
3. Add a terminal-independent cell/style intent layer.
4. Adapt scrollback, prompt, status, and overlays one component at a time;
   preserve existing YAML replay snapshots throughout migration.

The DSL is intentionally small: it describes structure and ownership without
hiding behavior in macros or introducing a second mutable state store.

DSL audit (2026-08-06): rechecked all declarative call sites and macro
expansions. `view!` owns element composition/slots and `layout_entries!` owns
fixed/grow entry declaration; both expand directly to renderer-independent
constructors and are used by the live chat layout. No repeated declarative
pattern currently justifies another macro without hiding allocation or state
ownership. The remaining DSL-adjacent work is terminal-independent reflow and
style-intent coverage, not additional syntax.
