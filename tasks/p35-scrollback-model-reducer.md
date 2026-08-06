# p35 — Extract the Scrollback reducer into the pure TUI model

Status: planned (2026-08-06)

## Why this is still open

`ScrollbackActor` publishes an immutable `runie-tui-model::FeedSnapshot`, but
its actor worker still reduces commands through `runie-tui::widgets::Scrollback`.
That widget contains both model state and Ratatui rendering, so the current
watch-channel boundary is safe for readers but not yet a complete declarative
model/render separation.

## Extraction slices

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
