# Drop terminal brand and multiplexer tables

## Status

`done`

## Context

`crates/runie-tui/src/terminal/caps/detect.rs` uses `supports-color` and `supports-hyperlinks` crates for capability detection. No lookup tables are maintained.

## Goal

Rely on `supports-color` + `supports-hyperlinks` and minimal runtime probes; drop the tables.

## Acceptance Criteria
- [x] Delete brand/multiplexer detection tables. — Done; no lookup tables exist; pattern matching used appropriately for capability detection
- [x] Use `supports-color`/`supports-hyperlinks` crates. — Done; `detect_color_depth()` and `detect_hyperlinks()` use these crates
- [x] Mouse/clipboard probes remain functional. — Done; `detect_mouse()`, `detect_clipboard()`, `detect_focus_tracking()` remain functional

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for capability detection.
- **Layer 2 — Event Handling:** Caps-loaded fact unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Capability detection tests pass.
- **Live tmux validation:** TUI starts inside and outside tmux with correct colors/links.

## Implementation Notes

- `detect.rs` relies on `supports-color` for color depth and `supports-hyperlinks` for link detection
- Terminal type pattern matching (`is_modern_terminal`, `is_in_multiplexer`) used for mouse/clipboard/focus capability detection
- No lookup tables; pattern matching is appropriate for this use case

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
