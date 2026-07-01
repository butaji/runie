# Execute five-round architecture review roadmap

## Status

`todo`

## Description

Track the integration of the five-round architecture review. Success metrics: fewer LOC, zero production files >500 lines, fewer custom modules, all async work observed.

## Acceptance criteria

- All Round 1–5 tasks are linked from this roadmap.
- Progress is tracked in `tasks/index.json`.
- Final verification runs the test suite and reports metrics.

## Tests

### Layer 1 — State/Logic
- Static metrics script counts LOC and files >500 lines.

### Layer 2 — Event Handling
- Smoke tests pass after each round.
