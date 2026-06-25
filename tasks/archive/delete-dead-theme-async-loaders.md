# Delete dead async theme loaders

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/theme/loader.rs` declares three async functions that are never used:

- `load_theme_raw_async(name: String) -> opaline::Theme` (line 31)
- `load_theme_async(name: String) -> opaline::Theme` (line 43)
- `load_theme_with_caps_async(name, caps) -> Theme` (line 61)

All three are flagged `#[warn(dead_code)]` by `cargo build`. The synchronous loaders in the same file are used by `theme::set_current_theme_with_caps` in `runie-tui/src/main.rs`. Either wire the async loaders in (preload before the first draw) or delete them.

## Acceptance Criteria

- [ ] One of:
  - All three async functions deleted; synchronous loaders continue to work as today.
  - All three async functions called by the theme bootstrap path so first-draw latency does not block on file IO.
- [ ] `cargo check --workspace` reports zero `dead_code` warnings on `loader.rs`.
- [ ] Live tmux validation: theme still applies on startup and on `SwitchTheme` event.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- N/A — pure code removal.

### Layer 3 — Rendering
- [ ] `theme_switch_renders_correct_palette` (already in `runie-tui/src/theme_tests.rs` or equivalent) still passes.

### Layer 4 — Smoke / Crash
- [ ] `cargo build --workspace` exits 0 with no new warnings.

## Files touched

- `crates/runie-tui/src/theme/loader.rs`

## Notes

- Default decision: delete. The synchronous path is on the bootstrap critical section (called from `render_task`), and the IO is small. If profile shows theme load blocking the first frame, revisit and wire async.
