# Grok-Style Mouse & Focus Terminal Init

**Status**: todo
**Milestone**: R4
**Category**: TUI / Terminal
**Priority**: P0

**Depends on**: (none)
**Blocks**: grok-mouse-hit-testing

## Description

Copy Grok Build's terminal initialization sequence so Runie enables the full
mouse + focus + paste protocol that modern terminals expect. Runie already has
capability detection in `crates/runie-term/src/terminal/caps.rs`; this task
replaces the minimal crossterm enable with Grok's raw sequence.

Grok's startup sequence (observed via raw PTY):

```text
ESC [?1049h   alternate screen
ESC [?1000h   legacy mouse press/release
ESC [?1002h   button-event tracking
ESC [?1003h   all motion events
ESC [?1015h   urxvt SGR coordinates
ESC [?1006h   standard SGR coordinates
ESC [?1004h   focus tracking
ESC [?2004h   bracketed paste
ESC [?25l     hide cursor
ESC [1 q      block cursor
ESC [>0q      cursor style variant
ESC [?2026h/l synchronized update begin/end (per frame)
OSC 0;title    set window title
```

## Acceptance Criteria

- [ ] `terminal_setup.rs` sends the raw enable sequence `CSI ?1000h ?1002h
  ?1003h ?1015h ?1006h` when mouse capability is not `None`.
- [ ] Cleanup on exit/shutdown/suspend sends the matching disables.
- [ ] Focus tracking (`?1004h`) and bracketed paste (`?2004h`) are enabled
  unconditionally (terminals that don't support them ignore the sequences).
- [ ] Synchronized updates (`?2026h/l`) wrap frame draws.
- [ ] Cursor is hidden during TUI operation and restored on exit.
- [ ] Window title is set to `Runie` on startup and updated with turn status.
- [ ] `cargo build --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn mouse_init_sequence_includes_all_grok_modes() {
    let mut buf = Vec::new();
    enable_mouse_grok_style(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[?1000h"));
    assert!(s.contains("\x1b[?1002h"));
    assert!(s.contains("\x1b[?1003h"));
    assert!(s.contains("\x1b[?1015h"));
    assert!(s.contains("\x1b[?1006h"));
}

#[test]
fn cleanup_sequence_disables_all_modes() {
    let mut buf = Vec::new();
    disable_mouse_grok_style(&mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("\x1b[?1003l"));
    assert!(s.contains("\x1b[?1006l"));
    assert!(s.contains("\x1b[?2004l"));
    assert!(s.contains("\x1b[?25h")); // show cursor
}
```

### Layer 4 — Smoke

```bash
# Run binary, capture raw output, verify enable sequence appears.
```

## Files touched

- `crates/runie-term/src/terminal_setup.rs`
- `crates/runie-term/src/terminal/mouse.rs` (new or extend)
- `crates/runie-term/src/effects/suspend.rs`
- `crates/runie-term/src/main.rs`

## Out of scope

- Interpreting mouse coordinates (covered by `grok-mouse-hit-testing`).
- Changing cursor color or shape beyond show/hide/block.
