# TUI replay conversations

## Objective

Use recorded SSE fixtures to drive real `runie-tui` tmux sessions and verify
streaming, multi-turn, tool-call, and reasoning rendering without API keys or
network access.

## Why this matters

Mock providers cover basic UI behavior but cannot exercise realistic provider
responses (streaming deltas, tool calls, thinking blocks, multi-turn context).
Replay fixtures fill that gap while keeping tests deterministic.

## Fixtures available

All fixtures live in `runie-tests/fixtures/{openai,anthropic}/`. Key scenarios:

| Scenario | Protocol | Fixture(s) |
|---|---|---|
| simple text | openai | `opencode_go_kimi_k2_6_simple.sse` |
| simple text | anthropic | `opencode_go_minimax_m3_simple.sse` |
| tool call | openai | `opencode_go_deepseek_v4_flash_tool.sse` |
| tool call | anthropic | `opencode_go_minimax_m3_tool.sse` |
| multi-tool | openai | `opencode_go_deepseek_v4_flash_multi_tool.sse` |
| multi-tool | anthropic | `opencode_go_minimax_m3_multi_tool.sse` |
| reasoning | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` |
| reasoning | anthropic | `opencode_go_minimax_m3_reasoning.sse` |
| math chain | openai | `opencode_go_kimi_k2_6_multiturn_math_chain_turn{1,2}.sse` |
| weather chain | anthropic | `opencode_go_minimax_m3_multiturn_weather_chain_turn{1,2}.sse` |
| read/summarize/follow-up | openai | `opencode_go_deepseek_v4_flash_multiturn_read_summarize_followup_turn{1,2}.sse` |
| reasoning follow-up | openai | `opencode_go_deepseek_v4_flash_multiturn_reasoning_followup_turn{1,2}.sse` |
| multi-tool then compare | openai | `opencode_go_deepseek_v4_flash_multiturn_multi_tool_then_compare_turn{1,2}.sse` |
| clarification | openai | `opencode_go_deepseek_v4_flash_multiturn_clarification_turn{1,2}.sse` |

## Required runie changes

- The replay provider is already implemented in `runie/crates/runie-provider/src/replay.rs`.
- TUI must correctly render streamed `TextDelta`, `ThinkingDelta`, `ToolCallStart`,
  `ToolResult`, and `Finish` events.
- Static fake tool registry must supply outputs for `get_weather`, `read_file`,
  and `list_dir` so tool-call fixtures complete end-to-end.

## Test scenarios

1. **Simple streaming text**
   - Setup: `AppTest::mock()` with `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_kimi_k2_6_simple.sse`.
   - Keys: `type 'say ok' press Enter wait_for_idle`
   - Assert: pane contains `ok` and a turn-complete indicator.

2. **Anthropic simple text**
   - Setup: `RUNIE_REPLAY_PROTOCOL=anthropic RUNIE_REPLAY_FIXTURES=fixtures/anthropic/opencode_go_minimax_m3_simple.sse`.
   - Keys: `type 'say ok' press Enter wait_for_idle`
   - Assert: pane contains `ok`.

3. **Tool call renders**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_tool.sse`.
   - Keys: `type 'weather in Paris' press Enter wait_for_idle`
   - Assert: pane contains `get_weather` and the fake tool result (e.g. `22` or `sunny`).

4. **Multi-tool call renders**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/anthropic/opencode_go_minimax_m3_multi_tool.sse`.
   - Keys: `type 'weather in Paris and Berlin' press Enter wait_for_idle`
   - Assert: pane contains two `get_weather` references.

5. **Reasoning/thinking block renders**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_deepseek_v4_flash_reasoning.sse`.
   - Keys: `type 'what is 9 times 7' press Enter wait_for_idle`
   - Assert: pane contains thinking indicator and final answer `63`.

6. **Multi-turn math chain**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/openai/opencode_go_kimi_k2_6_multiturn_math_chain_turn1.sse,fixtures/openai/opencode_go_kimi_k2_6_multiturn_math_chain_turn2.sse`.
   - Keys: `type 'What is 2 + 2?' press Enter wait_for_idle type 'Multiply that by 3.' press Enter wait_for_idle`
   - Assert: pane contains `4` and `12`.

7. **Multi-turn weather chain with tool results**
   - Setup: `RUNIE_REPLAY_FIXTURES=fixtures/anthropic/opencode_go_minimax_m3_multiturn_weather_chain_turn1.sse,fixtures/anthropic/opencode_go_minimax_m3_multiturn_weather_chain_turn2.sse` plus `RUNIE_REPLAY_PROTOCOL=anthropic`.
   - Keys: `type 'What is the weather in Paris?' press Enter wait_for_idle type 'What about Berlin?' press Enter wait_for_idle`
   - Assert: pane contains `Paris` and `Berlin`.

## Edge / negative cases

- Fixture file missing: app shows clear error, does not hang.
- Wrong protocol hint: graceful fallback or error message.
- Cancel turn during replay: TUI returns to idle without corruption.

## Dependencies

- `black_box_replay_testing`
- `turn_lifecycle`
- `tool_output_rendering`

## Acceptance checklist

- [x] All scenarios are driven by recorded SSE fixtures under
      `runie-tests/fixtures/` via `RUNIE_REPLAY_FIXTURES`; `AppTest::mock()` is
      used only for the replay-provider setup, not for echo responses.
- [x] Every fixture listed in "Fixtures available" is exercised by at least one
      CLI or TUI test.
- [x] Each test uses a temp `$HOME` so config is isolated.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
- [x] Tests run deterministically on repeated runs without API keys.
