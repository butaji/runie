# Use bytesize and humantime for tool formatting

## Status

`todo`

## Context

`crates/runie-core/src/tool/format.rs:115-138` contains hand-rolled `format_bytes` and `format_duration` helpers. `bytesize` and `humantime` are small, standard crates that do this more robustly.

## Goal

Replace the custom formatters with `bytesize` and `humantime`, preserving the same output style for tool results.

## Acceptance Criteria

- [ ] Remove custom `format_bytes` / `format_duration`.
- [ ] Use `bytesize::ByteSize` and `humantime::format_duration`.
- [ ] Keep output strings identical (e.g., "1.2 KB", "3.4 s").
- [ ] All formatting tests pass.

## Design Impact

No change to TUI element design or composition. Only the numeric strings shown in tool results may become more accurate; the visual style is unchanged.

## Tests

- **Layer 1 — State/Logic:** Unit tests for byte/duration formatting edge cases.
- **Layer 2 — Event Handling:** Tool result events carry formatted strings.
- **Layer 3 — Rendering:** `TestBackend` tool output snapshots match.
- **Layer 4 — E2E:** Headless CLI tool result formatting is unchanged.
- **Live tmux validation:** Ask the agent to run `ls -lh` or a timed command; output formatting matches.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
