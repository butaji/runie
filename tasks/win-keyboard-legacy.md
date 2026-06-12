# Windows Legacy Console Keyboard Support

**Status**: done
**Milestone**: R3
**Category**: Input & Commands

## Description

Crossterm's progressive keyboard enhancement flags (kitty keyboard protocol)
are not supported by the legacy Windows console API, which is used on
Windows ARM64. `PushKeyboardEnhancementFlags` fails during terminal setup
and prevents `runie` from starting. Make the enhancement optional so the
terminal still initializes on legacy Windows consoles.

## Acceptance Criteria

- [x] `setup_terminal` ignores unsupported keyboard enhancement errors
- [x] Suspend/resume path ignores unsupported keyboard enhancement errors
- [x] Keyboard enhancement is still enabled on terminals that support it
- [x] `runie-term` compiles on Windows ARM64

## Tests

### Layer 1 — State/Logic
- [x] `push_keyboard_enhancement_flags_writes_sequence` — helper emits the expected CSI sequence when the writer supports it
- [x] `push_keyboard_enhancement_flags_error_is_err` — helper surfaces errors from unsupported writers

### Layer 2 — Event Handling
- [ ] (not applicable; terminal setup is not event-driven)

### Layer 3 — Rendering
- [ ] (not applicable; no widget output changes)

### Layer 4 — Smoke
- [ ] (to be validated on Windows ARM64 hardware/VM)
