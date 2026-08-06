# p35 — Extract the Scrollback reducer into the pure TUI model

Status: in_progress (2026-08-06)

## Why this is still open

`ScrollbackActor` publishes an immutable `runie-tui-model::FeedSnapshot`, but
its actor worker still reduces commands through `runie-tui::widgets::Scrollback`.
That widget contains both model state and Ratatui rendering, so the current
watch-channel boundary is safe for readers but not yet a complete declarative
model/render separation.

## Extraction slices

### Slice 1 complete: animation ownership

`FeedNavigation` now owns the animation frame and pure advance/reset
transitions in `runie-tui-model`; `Scrollback` no longer stores a separate
renderer-local animation counter. Model tests cover frame advancement and
reset semantics, while the existing visual animation suite remains the
terminal-level oracle.

1. Move reducer state and pure transitions into a model-owned `FeedState`.
   Preserve line identity, tool-row ownership, folds, activity groups,
   workflow projections, selection, and follow/scroll semantics.
2. Make `FeedState::snapshot()` produce `FeedSnapshot` directly, including all
   facts needed by Grok card classification and animation demand.
3. Change `ScrollbackActor` to own `FeedState` and publish only its snapshot;
   retain a temporary `Scrollback::from_model_snapshot` adapter for rendering.
4. Move pure geometry/row expansion helpers into model or render-neutral
   modules; leave Ratatui buffers, styles, and terminal capabilities in the
   renderer.
5. Add YAML replay assertions for each transition family and retain the
   complete-screen four-size visual oracle as a separate rendering gate.

## Acceptance

- No actor worker constructs or mutates `Scrollback`.
- `runie-tui-model` has reducer tests for every `ScrollbackMsg` family.
- Existing YAML/replay and visual suites remain green without fixture-specific
  Rust code.
- `just ci` and the source-backed Pi/Grok validators pass.

### Slice 2 complete: follow/autoscroll ownership

`autoscroll` and `follow_latest_user` now live in `FeedNavigation`; append,
explicit-follow, reveal, selection, scroll, snapshot, and render paths all
use that model-owned value. The YAML `visual-tool-update` follow assertion
and the full visual matrix verify the adapter round-trip.

### Slice 3 complete: scroll offset ownership

`scroll_offset` now lives in `FeedNavigation` as well. Explicit scroll,
reveal-to-latest, append-tail following, physical-row clamping, selection
reveal, snapshot rehydration, and YAML scroll assertions use that single
model-owned field; rendering still performs only terminal-size clamping.

### Slice 4 complete: selection ownership

`selected_tool_id` and `selected_entry` now live in `FeedNavigation`. Tool and
entry navigation, dense-group reveal, selected-row rendering, snapshot
rehydration, and compatibility accessors all use the model-owned values. The
existing selection and dense-group visual/replay tests pass unchanged.

### Slice 5 complete: fold ownership

`reasoning_expanded` and `activity_expanded` now live in `FeedNavigation`.
Reasoning/activity reducers, fold rendering, snapshot rehydration, and the
existing expanded/collapsed YAML and visual cases use the model-owned flags.

### Slice 6 complete: tool display-mode ownership

The reducer-owned tool mode map now lives in `FeedNavigation`. Default mode
selection, explicit mode changes, fold cycling, typed-card projection,
snapshot rehydration, and specialized tool rendering all consume that one
model fact; the complete specialized-tool YAML and visual suite passes.

### Slice 7 complete: theme identity ownership

`ThemeKind` now lives in `FeedNavigation` as a model fact. Theme events,
snapshot rehydration, feed projections, and all day/night/terminal-native
rendering paths use that fact; only semantic-token-to-terminal-style
resolution remains in the renderer.

### Slice 8 complete: prompt timestamp ownership

The optional prompt timestamp now lives in `FeedNavigation`. Timestamp event
reduction, snapshot rehydration, live clock placement, and wrapped user-row
rendering consume that model fact; timestamped submission visual tests pass.
