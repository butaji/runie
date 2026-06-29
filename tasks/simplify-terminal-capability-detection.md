# Simplify terminal capability detection

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Terminal capability detection (`NO_COLOR`, color level, truecolor, hyperlinks) is implemented with custom environment-variable parsing and duplicated across the TUI crate. Replace it with the `supports-color` and `supports-hyperlinks` ecosystem crates and a single `TermCaps` snapshot.

## Acceptance Criteria

- [ ] `supports-color` determines color level instead of custom env parsing.
- [ ] `supports-hyperlinks` determines link support.
- [ ] A single `TermCaps` struct is computed once at startup and passed down.
- [ ] Custom `term_caps.rs` env parsing is deleted.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `term_caps_from_supports_color` — `TermCaps` reflects the crate's reported color level.
- [ ] `term_caps_respects_no_color` — `NO_COLOR=1` disables color.

### Layer 2 — Event Handling
- [ ] N/A — capability detection is not event-driven.

### Layer 3 — Rendering
- [ ] `render_uses_term_caps` — a `TestBackend` buffer shows no ANSI escapes when color is disabled.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — terminal capability is local UI state.

## Files touched

- `crates/runie-tui/src/terminal/caps/mod.rs`
- `crates/runie-tui/src/terminal/caps/detect.rs`
- `crates/runie-tui/src/terminal/caps/tests.rs`
- `crates/runie-tui/src/ui/*.rs`
- `Cargo.toml`

## Notes

- The `NO_COLOR` spec must still be honored; `supports-color` handles this, but add a test to be sure.
- Keep `TermCaps` cheap to clone (`Copy` or `Arc`).
