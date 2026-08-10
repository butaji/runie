# Runie delivery plan

This directory contains the current implementation plan and durable findings.
Historical task files were consolidated on 2026-08-09 because they described
completed work, duplicated one another, or tracked obsolete implementation
boundaries.

## Canonical files

- [`plan.md`](plan.md) — current workstreams, acceptance criteria, and next steps.
- [`findings.md`](findings.md) — source-backed architectural and parity findings.
- [`index.json`](index.json) — machine-readable summary for tooling.

Completed work is documented in the code, tests, and product documentation;
it is not represented as a separate closed task file.

## Working rules

- State belongs to one actor and crosses boundaries through events.
- Rendering is pure, reactive, and non-blocking.
- Replay and unit tests describe behavior as event sequences where practical.
- Every Rust change must pass formatting, workspace checks, tests, and the
  repository structural lint gate.
- A parity claim requires source evidence, a deterministic fixture or capture,
  and an explicit acceptance criterion.
