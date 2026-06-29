# Unify core and TUI line-count computation

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: replace-custom-helpers-with-crates
**Blocks**: none

## Description

Line wrapping and line-count logic exist in both `runie-core` and `runie-tui`, producing inconsistent message heights and duplicated code. Unify the computation in one place, either a shared helper in `runie-core` or the `textwrap` crate used consistently.

## Acceptance Criteria

- [ ] Exactly one source of truth computes wrapped line counts.
- [ ] TUI message/diff views use the core helper or `textwrap`.
- [ ] Duplicate wrapping logic in TUI is deleted.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `line_count_matches_textwrap` — core helper produces the same count as `textwrap` for a grid of inputs.
- [ ] `wrapped_height_for_message` — message height calculation matches manual expectation.

### Layer 2 — Event Handling
- [ ] N/A — line counting is not event-driven.

### Layer 3 — Rendering
- [ ] `message_list_scroll_height` — a `TestBackend` render of a message list shows the expected wrapped height.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A — pure layout utility.

## Files touched

- `crates/runie-core/src/layout.rs`
- `crates/runie-tui/src/message/wrap.rs`
- `crates/runie-tui/src/ui/messages/lines.rs`
- `Cargo.toml`

## Notes

- Prefer `textwrap` if it already covers the width/unicode cases; otherwise keep a thin core wrapper.
- Account for ANSI escape sequences so styled text does not wrap early.
