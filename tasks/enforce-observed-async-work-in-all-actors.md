# Enforce observed async work in all actors

## Status

`todo`

## Description

The SSOT ADR requires every spawned task to have an owner. This task adds an invariant check (code review + optional lint) that no unbounded fire-and-forget `tokio::spawn` exists in actor code.

## Acceptance criteria

- All production actor code either awaits spawned work or stores the `JoinHandle`/`JoinSet`.
- A CI grep/lint fails on new unbounded `tokio::spawn` in actor modules.

## Tests

### Layer 1 — State/Logic
- Static analysis / lint test verifies no orphan spawns in actor modules.
