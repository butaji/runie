# Replace ANSI256 lookup table with ansi_colours

## Status

`todo`

## Context

`crates/runie-tui/src/quantize.rs` maintains a hand-written 256-byte `ANSI256_TO_16` table.

## Goal

Compute nearest basic ANSI color from RGB via `ansi_colours::ansi256_from_rgb` and a small distance helper.

## Acceptance Criteria
- [ ] Delete lookup table.
- [ ] Use `ansi_colours` + Euclidean distance over 16 standard colors.
- [ ] Snapshots updated or preserved.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests compare old and new mapping.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests pass.
- **Layer 4 — E2E:** N/A.
- **Live tmux validation:** Theme colors render correctly on 16-color terminals.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
