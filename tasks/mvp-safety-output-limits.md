# Output size limits

**Status**: done

**Milestone**: MVP

**Category**: Safety

## Description

Limit tool output sizes.

## Acceptance Criteria

- [x] Max bytes limit (DEFAULT_MAX_BYTES: 50KB)
- [x] Max lines limit (DEFAULT_MAX_LINES: 2000)
- [x] Truncation with indicator (TruncatedOutput::was_truncated)
- [x] Per-tool limits (TruncationPolicy configurable)

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
