# Wire remaining OpenAI DeepSeek multi-turn fixtures

## Objective

Add black-box test coverage for the remaining DeepSeek multi-turn fixtures that exist on disk but are not yet referenced by any test.

## Fixtures to wire

```text
fixtures/openai/opencode_go_deepseek_v4_flash_multiturn_math_chain_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_flash_multiturn_math_chain_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_clarification_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_clarification_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_multi_tool_then_compare_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_multi_tool_then_compare_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_read_summarize_followup_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_read_summarize_followup_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_reasoning_followup_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_reasoning_followup_turn2.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse
fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse
```

## Implementation

Add tests to `tests/cli_replay.rs` and `tests/tui_replay_conversations.rs`. Do not create new test files.

### CLI tests in `tests/cli_replay.rs`

For each multi-turn pair, append a `#[tokio::test]` function:

```rust
#[tokio::test]
async fn deepseek_v4_pro_multiturn_math_chain_replays() {
    test_cli()
        .fixtures([
            "openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn1.sse",
            "openai/opencode_go_deepseek_v4_pro_multiturn_math_chain_turn2.sse",
        ])
        .args(["print", "What is 2 + 2?", "Multiply that by 3."])
        .assert()
        .await
        .unwrap()
        .stdout(contains("4"))
        .stdout(contains("12"))
        .success();
}
```

Repeat for `clarification`, `multi_tool_then_compare`,
`read_summarize_followup`, `reasoning_followup`, and `weather_chain`. Use the
same two-prompt pattern already present in `tests/cli_replay.rs` for
`deepseek_v4_pro_multiturn_weather_chain`.

For the missing `deepseek_v4_flash` math chain, append:

```rust
#[tokio::test]
async fn deepseek_v4_flash_multiturn_math_chain_replays() {
    test_cli()
        .fixtures([
            "openai/opencode_go_deepseek_v4_flash_multiturn_math_chain_turn1.sse",
            "openai/opencode_go_deepseek_v4_flash_multiturn_math_chain_turn2.sse",
        ])
        .args(["print", "What is 2 + 2?", "Multiply that by 3."])
        .assert()
        .await
        .unwrap()
        .stdout(contains("4"))
        .stdout(contains("12"))
        .success();
}
```

### TUI tests in `tests/tui_replay_conversations.rs`

For each multi-turn pair, append a `#[tokio::test]` function that sends both
prompts sequentially:

```rust
#[tokio::test]
async fn tui_deepseek_v4_pro_multiturn_weather_chain_renders() {
    test_tui()
        .fixtures([
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse",
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse",
        ])
        .type_keys("What is the weather in Paris?")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .type_keys("What about Berlin?")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("Paris"))
        .assert(contains("Berlin"));
}
```

## Dependencies

- `cli_replay_conversations`
- `tui_replay_conversations`
- `black_box_replay_dsl`

## Acceptance checklist

- [x] All 14 fixtures above are referenced by at least one test.
- [x] `cargo test --test cli_replay` passes for newly added CLI tests.
- [x] `cargo test --test tui_replay_conversations` passes for newly added TUI tests.
- [x] No fixture remains unwired after this task.
