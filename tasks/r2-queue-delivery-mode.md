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
