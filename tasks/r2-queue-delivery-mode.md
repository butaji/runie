# Queue delivery mode configuration

**Status**: todo

**Milestone**: R2

**Category**: Input & Commands

## Description

Configure how queued messages are delivered to the LLM. Steering and follow-up modes can be set independently.

## Modes

- **one-at-a-time** (default): Each queued message triggers a separate LLM call. The agent waits for a response before processing the next message.
- **all**: All queued messages are delivered together in a single LLM call.

## Configuration

Set via `/settings` or `settings.json`:

```json
{
  "steeringMode": "one-at-a-time" | "all",
  "followUpMode": "one-at-a-time" | "all"
}
```

## Acceptance Criteria

- [ ] steeringMode configuration (one-at-a-time / all)
- [ ] followUpMode configuration (one-at-a-time / all)
- [ ] QueueAgent respects delivery mode when emitting SpawnAgent
- [ ] ConfigAgent loads and validates delivery mode settings
- [ ] Default to one-at-a-time for both modes

## Tests

Required per AGENTS.md. See `tasks/TEMPLATE.md` for the full format.

- [ ] Layer 1 — State/logic tests (pure functions, no ratatui)
- [ ] Layer 2 — Event handling tests (crossterm events → state transitions)
- [ ] Layer 3 — Rendering tests (TestBackend + Buffer assertions) if TUI-related
- [ ] Layer 4 — Smoke tests (tmux) if async/event logic changes
