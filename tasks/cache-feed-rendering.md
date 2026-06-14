# Cache Feed Rendering and Theme Reload

**Status**: done
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P1

## Description

`runie_tui::ui::draw_snapshot` reloaded the theme from disk and re-rendered every element (markdown parse, code highlight, line wrapping) on every frame. This was `O(feed_size)` per frame.

## Acceptance Criteria

- [x] Theme is cached by name; `set_current_theme` is a no-op if the theme is unchanged.
- [x] Per-element rendered lines / markdown blocks are cached by `(content_hash, content_width)`.
- [x] Only visible/changed elements are re-rendered.
- [x] Performance does not degrade with long sessions.

## Tests

### Layer 1 — State/Logic
- [x] `theme_cache_returns_same_instance`.
- [x] `element_render_cache_hits_for_same_width_and_content`.

### Layer 3 — Rendering
- [x] `long_feed_renders_in_reasonable_time`.
