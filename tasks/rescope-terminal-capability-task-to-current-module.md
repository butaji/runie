# Rescope terminal capability task to current module layout

**Status**: done
**Milestone**: R6
**Category": TUI / Rendering
**Priority": P2

**Depends on**: none
**Blocks**: none

## Description

`simplify-terminal-capability-detection.md` references deleted files, but the current code lives in `crates/runie-tui/src/terminal/caps/mod.rs` and `detect.rs`. This task assessed the current module and decided the fate of the remaining heuristics.

### Decision Summary

After reviewing the current implementation, all remaining heuristics are **kept**:

| Heuristic | Purpose | Duplicates crate? | Decision |
|-----------|---------|-------------------|----------|
| `detect_brand` | Identify terminal emulator | No | **Keep** |
| `detect_multiplexer` | Detect TMUX/Zellij/Screen | No | **Keep** |
| `detect_mouse` | Determine mouse protocol level | No | **Keep** |
| `detect_clipboard` | OSC 52 support heuristic | No | **Keep** |
| `detect_focus_tracking` | Focus event support | No | **Keep** |
| `detect_unicode` | UTF-8 locale detection | No | **Keep** |

The current implementation already uses:
- `supports-color` for color level detection
- `supports-hyperlinks` for hyperlink support

The heuristics provide sensible defaults based on terminal brand detection without duplicating any crate functionality.

## Acceptance Criteria

- [x] Current module structure documented
- [x] Heuristics assessed for crate duplication
- [x] All heuristics deemed necessary and kept
- [x] `simplify-terminal-capability-detection.md` updated

## Notes

- No code changes were required; documentation only.
- The current implementation is reasonable and does not need further simplification.
