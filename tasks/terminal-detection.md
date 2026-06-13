# Terminal Capability Detection

**Status**: in-progress
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: -

## Description

Runie currently enters the alternate screen and enables raw mode with no
knowledge of the surrounding terminal. This means it cannot adapt to
capability differences: a user on a 256-color terminal sees the same
24-bit theme as a user on iTerm2, and features like focus tracking or
OSC-52 clipboard are never enabled. The Grok TUI research identifies
**terminal detection + progressive enhancement** as the key polish
technique.

## What Was Done

- [x] Created `crates/runie-term/src/terminal/caps.rs` with heuristic
  detection of:
  - Terminal emulator brand (`TerminalBrand`)
  - Terminal multiplexer (`MultiplexerType`)
  - Truecolor support (`truecolor`)
  - Mouse protocol level (`MouseCapability`) — detected but not enabled
  - OSC 52 clipboard support (`clipboard`)
  - Focus tracking support (`focus_tracking`)
  - Unicode locale (`unicode`)
- [x] `detect_capabilities(env)` is pure over a `HashMap<String, String>`
  snapshot for deterministic Layer 1 tests.
- [x] `detect_capabilities_from_env()` convenience wrapper reads
  `std::env::vars()`.
- [x] Wired detection into `terminal_setup::setup_terminal()`; it now
  returns `(Terminal, TerminalCapabilities)`.
- [x] Progressive enhancement: focus tracking is enabled with
  `EnableFocusChange` when detected as supported, and disabled in the
  cleanup `Drop` impl.
- [x] `restore_terminal_graphics()` takes capabilities so focus tracking
  is re-enabled after resume from `SIGTSTP`.
- [x] Mouse handling is intentionally **not** enabled per user request
  ("skip mouse"); capability is detected for future use.

## Acceptance Criteria

- [x] `cargo check -p runie-term` succeeds with no new warnings.
- [x] `cargo build -p runie-term` succeeds (lint rules pass).
- [x] `cargo test -p runie-term terminal` passes.
- [x] Focus tracking is enabled only when `focus_tracking` is true.
- [x] Mouse events are not emitted (mouse is detected but not enabled).

## Tests

### Layer 1 — State/Logic
- [x] `truecolor_from_colorterm` — `COLORTERM=truecolor` → truecolor.
- [x] `truecolor_from_24bit_colorterm` — `COLORTERM=24bit` → truecolor.
- [x] `truecolor_from_term_suffix` — `TERM=...-direct` → truecolor.
- [x] `no_truecolor_without_hints` — conservative default.
- [x] `brand_iterm2`, `brand_vscode`, `brand_windows_terminal`,
  `brand_kitty_from_term`, `brand_unknown_when_empty`.
- [x] `multiplexer_tmux`, `multiplexer_zellij`,
  `multiplexer_screen_from_term`, `multiplexer_none`.
- [x] `mouse_none_for_unknown_terminal`,
  `mouse_sgr_for_modern_terminals`, `mouse_sgr_for_multiplexers`.
- [x] `clipboard_true_for_supported_terminals`,
  `clipboard_false_for_unknown_terminal`.
- [x] `focus_tracking_for_modern_or_multiplexer`,
  `focus_tracking_false_for_unknown_bare_terminal`.
- [x] `unicode_from_lang`, `unicode_defaults_true_when_unset`,
  `non_unicode_locale`.
- [x] `full_detection_combines_fields` — end-to-end snapshot detection.
- [x] `default_capabilities_are_conservative`.

### Layer 2 — Event Handling
- [x] Existing `terminal_setup` tests still pass:
  - `push_keyboard_enhancement_flags_writes_sequence`
  - `push_keyboard_enhancement_flags_error_is_err`

### Layer 3 — Rendering
- [x] No rendering changes required for detection itself.

### Layer 4 — Smoke
- [ ] Run `./target/release/runie` in a real terminal and verify it
  starts without panic; `tmux` smoke test should pass.

## Notes

**Mouse is detected but not enabled.** The user explicitly asked to skip
mouse handling. The `MouseCapability` enum is in place so mouse can be
wired later without changing the capability structure.

**Truecolor does not yet change theme selection.** Opaline themes are
static today; a follow-up task can add 256-color quantization and switch
to a reduced palette when `truecolor == false`.

**Focus tracking events are consumed and ignored.** `keymap::convert_event`
returns `None` for `Event::FocusGained` / `Event::FocusLost`. A future task
can route these to `AppState` for pause/resume of animations or
notifications.

**Out of scope:**
- OSC 52 clipboard implementation (detection only).
- Theme quantization / fallback palette.
- Full mouse event routing and drag handling.

## Verification

```bash
# Compile clean
cargo check -p runie-term
cargo build -p runie-term

# Run detection tests
cargo test -p runie-term terminal

# Full runie-term unit suite (one pre-existing performance test is flaky
# on this machine and unrelated to this change)
cargo test -p runie-term --lib -- --skip test_render_performance_1000_messages
```
