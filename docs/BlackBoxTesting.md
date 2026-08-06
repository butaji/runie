# Black-Box Testing Guide

Replay tests are part of this repository and are driven by event fixtures.
For TUI behavior, use the YAML scenarios under
`crates/runie-tui/tests/e2e/` and the visual replay harness.

## Quick reference

From this repository root:

```bash
just ci
```

## Environment variables

| Variable | Description |
|----------|-------------|
| `RUNIE_REPLAY_FIXTURES` | Comma-separated list of fixture file paths |
| `RUNIE_REPLAY_PROTOCOL` | Protocol: `openai` or `anthropic` (auto-detected if not set) |

## Fixtures and scripts

- Replay fixtures: `tests/traces/` (`.sse` plus `.sse.yaml`)
- TUI scenarios: `crates/runie-tui/tests/e2e/*.yaml`
- Capture and parity tooling: `parity/` and the `justfile`

## Replay provider

Replay providers and state assertions are selected by the fixture runners;
there is no separate `runie-provider` crate in this workspace.
