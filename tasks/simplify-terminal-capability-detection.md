# Simplify terminal capability detection

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Terminal capability detection (`NO_COLOR`, color level, truecolor, hyperlinks) is implemented with custom environment-variable parsing and duplicated across the TUI crate. Replace it with the `supports-color` and `supports-hyperlinks` ecosystem crates and a single `TermCaps` snapshot.

## Acceptance Criteria

- [x] `supports-color` determines color level instead of custom env parsing.
- [x] `supports-hyperlinks` determines link support.
- [x] A single `TermCaps` struct is computed once at startup and passed down.
- [x] Custom `term_caps.rs` env parsing is deleted.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `term_caps_from_supports_color` — `TermCaps` reflects the crate's reported color level.
- [x] `term_caps_respects_no_color` — `NO_COLOR=1` disables color.

### Layer 2 — Event Handling
- [x] N/A — capability detection is not event-driven.

### Layer 3 — Rendering
- [x] `render_uses_term_caps` — a `TestBackend` buffer shows no ANSI escapes when color is disabled.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — terminal capability is local UI state.

## Files touched

- `crates/runie-tui/src/terminal/caps/mod.rs` — `TermCaps` struct and detection entry point
- `crates/runie-tui/src/terminal/caps/detect.rs` — detection helpers using `supports-color` and `supports-hyperlinks`
- `crates/runie-tui/src/terminal/caps/tests.rs` — comprehensive tests

## Notes

- The `NO_COLOR` spec is honored via `supports-color`.
- `TermCaps` is `Copy` and cheap to clone.
- Brand/multiplexer detection remains custom heuristics over an env snapshot for testability.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
