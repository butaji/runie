# CLI replay conversations

## Objective

Use recorded SSE fixtures to drive the compiled `runie-cli` binary and verify
streaming, single-turn, multi-turn, tool-call, and reasoning output without API
keys or network access.

## Why this matters

CLI replay tests are the simplest, fastest black-box coverage for the provider
layer. They verify that `runie-cli print`/`json` correctly consume fixtures via
`RUNIE_REPLAY_FIXTURES`, stream events, invoke the fake tool registry, and exit
cleanly.

## Fixtures available

All fixtures live in `runie-tests/fixtures/{openai,anthropic}/`. Key scenarios:

| Scenario | Protocol | Fixture(s) |
|---|---|---|
| simple text | openai | `opencode_go_deepseek_v4_flash_simple.sse` |
| simple text | anthropic | `opencode_go_minimax_m3_simple.sse` |
| tool call | openai | `opencode_go_deepseek_v4_flash_tool.sse` |
| tool call | anthropic | `opencode_go_minimax_m3_tool.sse` |
| multi-tool | openai | `opencode_go_deepseek_v4_flash_multi_tool.sse` |
| reasoning | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` |
| math chain | openai | `opencode_go_deepseek_v4_pro_multiturn_math_chain_turn{1,2}.sse` |
| weather chain | openai | `opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn{1,2}.sse` |

## Required runie changes

- The replay provider is already implemented in `runie/crates/runie-provider/src/replay.rs`.
- CLI must correctly stream `TextDelta`, `ThinkingDelta`, `ToolCallStart`,
  `ToolResult`, and `Finish` events to stdout/stderr.
- Static fake tool registry must supply outputs for `get_weather`, `read_file`,
  and `list_dir` so tool-call fixtures complete end-to-end.

## Test scenarios

1. **Simple streaming text**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_simple.sse`.
   - Args: `runie print "say ok"`
   - Assert: stdout contains `ok`; exit code is zero.

2. **Anthropic simple text**
   - Setup: `RUNIE_REPLAY_PROTOCOL=anthropic RUNIE_REPLAY_FIXTURES=fixtures/anthropic/opencode_go_minimax_m3_simple.sse`.
   - Args: `runie print "say ok"`
   - Assert: stdout contains `ok`.

3. **Tool call output**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_tool.sse`.
   - Args: `runie print "weather in Paris"`
   - Assert: stdout contains `get_weather` and the fake result (e.g. `22` or `sunny`).

4. **JSON tool call output**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_tool.sse`.
   - Args: `runie json --model opencode-go/deepseek-v4-flash "weather in Paris"`
   - Assert: stdout is valid JSON containing `get_weather`.

5. **Reasoning output**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_reasoning.sse`.
   - Args: `runie print "what is 9 times 7"`
   - Assert: stdout contains reasoning indicator and final answer `63`.

6. **Multi-turn math chain**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn1.sse,fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn2.sse`.
   - Args: `runie print "What is 2 + 2?" "Multiply that by 3."`
   - Assert: stdout contains `4` and `12`.

7. **Multi-turn weather chain**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse,fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse`.
   - Args: `runie print "What is the weather in Paris?" "What about Berlin?"`
   - Assert: stdout contains `Paris` and `Berlin`.

## Edge / negative cases

- Fixture file missing: non-zero exit code with clear error.
- Wrong protocol hint: graceful fallback or error message.
- Invalid API key replay fixture: non-zero exit, no leaked key value.

## Dependencies

- `black_box_replay_testing`

## Acceptance checklist

- [x] All scenarios pass with `find_runie_cli_binary()` and per-command env vars.
- [x] Each test uses a temp `$HOME` so config is isolated.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests run deterministically on repeated runs without API keys.
- [x] No secret values appear in assertion output.
