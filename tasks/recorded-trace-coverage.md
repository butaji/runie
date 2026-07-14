# Recorded trace coverage

## Objective

Use every recorded SSE fixture under `runie-tests/fixtures/` to drive both CLI
and TUI black-box tests, ensuring `runie` behaves correctly against real model
API behavior that has been captured and replayed.

## Why this matters

Recorded traces are the only way to test `runie` against realistic provider
responses (streaming deltas, tool calls, reasoning blocks, multi-turn context,
errors) without API keys or network calls. The black-box suite must exercise
*all* recorded traces for *all* supported models and scenarios.

## Recorded models and protocols

Fixtures are organized by provider protocol:

- `fixtures/openai/` — OpenAI-compatible traces (DeepSeek, Kimi, etc.)
- `fixtures/anthropic/` — Anthropic-compatible traces (MiniMax, etc.)

Each protocol contains simple, tool, multi-tool, reasoning, and multi-turn
recordings.

## Current coverage

All 131 recorded SSE fixtures are referenced by at least one black-box test.

| Protocol | Fixtures on disk | Wired | Unwired |
|---|---|---|---|
| `fixtures/openai/` | 77 | 77 | 0 |
| `fixtures/anthropic/` | 54 | 54 | 0 |
| **Total** | **131** | **131** | **0** |

"Wired" means the fixture path appears in at least one test in `tests/`.

## Required coverage matrix

| Flow | CLI | TUI |
|---|---|---|
| Simple streaming text | ✅ covered | ✅ covered |
| Tool call with fake registry | ✅ covered | ✅ covered |
| Multi-tool call | ✅ covered | ✅ covered |
| Reasoning / thinking block | ✅ covered | ✅ covered |
| Multi-turn math chain | ✅ covered | ✅ covered |
| Multi-turn weather chain | ✅ covered | ✅ covered |
| Read/summarize/follow-up | ✅ covered | ✅ covered |
| SSE error fixtures (rate limit, server error, context length, invalid key, model not found, stream error mid-response) | ✅ covered | ✅ covered |
| HTTP-status error fixtures (429, 500, 502, 503, 401, 403) | ✅ covered | ✅ covered |

## Test design rules

- Every fixture must be referenced by at least one test.
- CLI tests use `test_cli().fixture(...).args(...).assert()`.
- TUI tests use `test_tui().fixture(...)` and assert on captured tmux pane text.
- TUI interactive tests use `AppTest::mock()` with the new helpers
  (`expect_response`, `expect_selected_row`, `request_tool_permission`, etc.).
- No test may call a live model API or require an API key.
- All tool-call fixtures rely on the static fake tool registry for deterministic
  tool results.

## Sub-tasks

| Task | Fixtures covered |
|---|---|
| `tasks/wire-anthropic-minimax-remaining-fixtures.md` | `minimax_m2_5` simple/tool, `minimax_m2_7` reasoning, `minimax_m3` multi-turn |
| `tasks/wire-anthropic-qwen-remaining-fixtures.md` | `qwen3_5_plus`, `qwen3_6_plus`, `qwen3_7_max`, `qwen3_7_plus` |
| `tasks/wire-openai-deepseek-remaining-multiturn-fixtures.md` | `deepseek_v4_flash` math chain, `deepseek_v4_pro` all multi-turn |
| `tasks/wire-openai-glm-remaining-fixtures.md` | `glm_5`, `glm_5_1`, `glm_5_2` |
| `tasks/wire-openai-kimi-remaining-fixtures.md` | `kimi_k2_6`, `kimi_k2_7_code` |
| `tasks/wire-openai-mimo-remaining-fixtures.md` | `mimo_v2_5`, `mimo_v2_5_pro` |

## Dependencies

- `cli_replay_conversations`
- `tui_replay_conversations`
- `error_state_rendering`
- `black_box_replay_dsl`
- `wire-anthropic-minimax-remaining-fixtures`
- `wire-anthropic-qwen-remaining-fixtures`
- `wire-openai-deepseek-remaining-multiturn-fixtures`
- `wire-openai-glm-remaining-fixtures`
- `wire-openai-kimi-remaining-fixtures`
- `wire-openai-mimo-remaining-fixtures`

## Acceptance checklist

- [x] Every fixture in `runie-tests/fixtures/` is referenced by at least one black-box test.
- [x] `cargo test --test cli_replay` passes for all CLI fixtures.
- [x] `cargo test --test tui_replay_conversations` passes for all TUI fixtures.
- [x] `cargo test --test error_recovery` passes for retry/recovery tests.
- [x] A single command lists every fixture and the test that covers it
      (`tasks/fixture-coverage-report.md`).
- [x] Coverage report includes model names, protocols, scenario counts, and error
      categories (`tasks/fixture-coverage-report.md`).
