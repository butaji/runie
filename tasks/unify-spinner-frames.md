# Unify Braille Spinner Frame Definitions

**Status**: done
**Milestone**: R3
**Category**: TUI Rendering
**Priority**: P2

## Description

Two spinner frame arrays are defined in different crates:

- `crates/runie-core/src/model.rs` — 12-frame `SPINNER_CHARS`.
- `crates/runie-tui/src/glyphs.rs` — 10-frame `SPINNER_FRAMES` / `SPINNER_FRAMES_REVERSE`.

The TUI currently uses the core frames via `state.spinner_frame()`. The duplicate in
`glyphs.rs` should be removed and all callers should use the core definition.

## Acceptance Criteria

- [x] A single source of spinner frames exists (keep the 12-frame set in `runie-core`).
- [x] `crates/runie-tui/src/glyphs.rs` no longer defines spinner frames.
- [x] All callers reference the core frames via `snap.spinner_frame`.
- [x] `cargo build --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `spinner_frame_cycles_twelve_times` — frame index wraps after 12.

### Layer 3 — Rendering
- [x] `thinking_indicator_uses_braille_spinner` — status text contains a Braille character.

## Files touched

- `crates/runie-core/src/model.rs`
- `crates/runie-tui/src/glyphs.rs`
- Any TUI callers of spinner frames.

## Out of scope

- Changing the animation speed or frame set.
