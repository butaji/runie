# Black-Box Testing Guide

Black-box replay tests for Runie live in the parent `runie-tests` repository.
This file is a pointer; see `runie-tests/docs/black-box-replay-testing.md` for
the full guide.

## Quick reference

From the `runie-tests` repo root:

```bash
cd runie
RUNIE_REPLAY_FIXTURES=../fixtures/openai/opencode_go_deepseek_v4_flash_simple.sse \
  cargo run -p runie-cli -- print "hello"
```

## Environment variables

| Variable | Description |
|----------|-------------|
| `RUNIE_REPLAY_FIXTURES` | Comma-separated list of fixture file paths |
| `RUNIE_REPLAY_PROTOCOL` | Protocol: `openai` or `anthropic` (auto-detected if not set) |

## Fixtures and scripts

- Fixtures: `runie-tests/fixtures/{openai,anthropic}/`
- Recording scripts: `runie-tests/scripts/record_opencode_go.py` and `record_opencode_go_multiturn.py`
- Black-box tests: `runie-tests/tests/replay_blackbox.rs`
- Task plans: `runie-tests/tasks/black-box-replay-testing.md` and `black-box-replay-dsl.md`

## Replay provider

The `ReplayProvider` implementation is in
`crates/runie-provider/src/replay.rs`. It is selected when
`RUNIE_REPLAY_FIXTURES` is set.
