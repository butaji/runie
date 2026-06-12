# <Feature Name>

**Status**: todo
**Milestone**: MVP | R1 | R2 | R3
**Category**: Core Architecture | Tools | TUI Rendering | Input & Commands | Sessions | Configuration | Safety

## Description

One-paragraph description of what this feature does and why it exists.
Keep it focused. If you need more than one paragraph, the task is too big.

## Acceptance Criteria

- [ ] Functional requirement 1
- [ ] Functional requirement 2
- [ ] Functional requirement 3

## Tests

### Layer 1 — State/Logic
- [ ] `test_description_of_pure_behavior` — verifies state transition without ratatui

### Layer 2 — Event Handling
- [ ] `test_event_produces_expected_state` — feeds crossterm events into handler

### Layer 3 — Rendering
- [ ] `test_widget_renders_expected_buffer` — TestBackend + Buffer assertion

### Layer 4 — Smoke (if async/event logic changes)
- [ ] `smoke_test_name.sh` — tmux script, no sleep assertions, checks for panics/stuck timers

## Notes

- Links to ADRs, prior art, or implementation hints
- Explicitly call out what is **out of scope**
