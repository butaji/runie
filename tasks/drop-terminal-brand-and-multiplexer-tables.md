# Drop terminal brand and multiplexer tables

## Status

`todo`

## Context

`crates/runie-tui/src/terminal/caps/detect.rs` keeps custom brand/multiplexer lookup tables over `TERM_PROGRAM`, `TMUX`, etc.

## Goal

Rely on `supports-color` + `supports-hyperlinks` and minimal runtime probes; drop the tables.

## Acceptance Criteria
- [ ] Delete brand/multiplexer detection tables.
- [ ] Use `supports-color`/`supports-hyperlinks` crates.
- [ ] Mouse/clipboard probes remain functional.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for capability detection.
- **Layer 2 — Event Handling:** Caps-loaded fact unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Capability detection tests pass.
- **Live tmux validation:** TUI starts inside and outside tmux with correct colors/links.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
