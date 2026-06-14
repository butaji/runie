# OSC 52 Clipboard Integration

**Status**: done
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0

## Resolution

OSC 52 clipboard integration is fully implemented:
- `crates/runie-term/src/terminal/clipboard.rs` — OSC 52 sequences
- `crates/runie-term/src/effects/clipboard.rs` — wired via effects module
- `/copy` command emits `CopyToClipboard` event
- `terminal_caps.clipboard` gates the OSC 52 escape sequence

Layer 4 smoke test (manual: copy text in a real terminal) remains unchecked.

Archived in tasks/archive/.
