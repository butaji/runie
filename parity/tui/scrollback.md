# Scrollback

## Purpose

Render measured entries, not an unstructured text buffer. Each entry owns its
display mode, fold state, vertical padding, background, and timestamp policy.

## Grok construction

Sources: `src/scrollback/entry.rs`, `block.rs`, `scrollback_pane.rs`, and
`render.rs`.

The layout pass computes heights first, applies sticky-header/viewport rules,
then paints only the selected range. Collapsed activity is a grouped block;
member content is hidden while the group remains the navigation unit.

## Runie mapping

`Scrollback` is the pure reducer state; `ScrollbackActor` owns it and publishes
watch snapshots. `physical_rows` is the current compatibility measurement
pass and is being replaced with per-block height metadata.
`runie-tui-model::sticky::compute_sticky_layout` now provides pure prompt
header collapse/push math; renderer integration remains the next boundary.

## Acceptance

`visual-grok-feed.yaml` must remain a strict full-screen reference. No fixture
may be weakened to hide a scroll offset or geometry mismatch.
