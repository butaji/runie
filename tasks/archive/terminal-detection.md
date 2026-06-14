# Terminal Capability Detection

**Status**: done
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0

## Resolution

Implemented in `crates/runie-term/src/terminal/caps.rs`:
- `TerminalCapabilities` struct with brand, truecolor, mouse, clipboard, focus_tracking, unicode detection
- `detect_capabilities(env: HashMap<String, String>)` — pure function, deterministic Layer 1 tests
- Progressive enhancement wired into `setup_terminal()`
- Focus tracking enabled/disabled via `restore_terminal_graphics()`

Layer 4 smoke test (manual: run binary in tmux) was not automated.

Archived in tasks/archive/.
