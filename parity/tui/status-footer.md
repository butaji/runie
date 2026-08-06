# Status and footer

## Contract

Status is an actor-owned projection of events. The view renders phase,
spinner, elapsed, usage, stop reason, and key hints as one pure row.

## States

Ready, thinking, streaming, loading, waiting, error, and completed.

## Acceptance

`visual-status-working.yaml`, `visual-grok-waiting.yaml`, and status snapshot
tests compare exact glyphs, colors, modifiers, and padding.
