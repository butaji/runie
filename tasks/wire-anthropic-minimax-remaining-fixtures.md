# Wire remaining Anthropic MiniMax fixtures

## Objective

Add black-box test coverage for the remaining Anthropic-protocol MiniMax fixtures that are on disk but not yet referenced by any test.

## Fixtures to wire

```text
fixtures/anthropic/opencode_go_minimax_m2_5_simple.sse
fixtures/anthropic/opencode_go_minimax_m2_5_tool.sse
fixtures/anthropic/opencode_go_minimax_m2_7_reasoning.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_clarification_turn1.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_clarification_turn2.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_multi_tool_then_compare_turn1.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_multi_tool_then_compare_turn2.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_read_summarize_followup_turn1.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_read_summarize_followup_turn2.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_reasoning_followup_turn1.sse
fixtures/anthropic/opencode_go_minimax_m3_multiturn_reasoning_followup_turn2.sse
```

## Implementation

Add tests to the existing replay test files. Do not create new test files.

### CLI tests in `tests/cli_replay.rs`

For each single-turn fixture, append a `#[tokio::test]` function:

```rust
#[tokio::test]
async fn minimax_m2_5_simple_replays() {
    test_cli()
        .fixture("anthropic/opencode_go_minimax_m2_5_simple.sse")
        .protocol("anthropic")
        .args(["print", "say ok"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("ok"))
        .success();
}
```

Repeat for `minimax_m2_5_tool.sse` (assert `get_weather` and `22` or `sunny`) and
`minimax_m2_7_reasoning.sse` (assert reasoning indicator and `63`).

For each multi-turn pair in `minimax_m3`, append a `#[tokio::test]` function:

```rust
#[tokio::test]
async fn minimax_m3_multiturn_clarification_replays() {
    test_cli()
        .fixtures([
            "anthropic/opencode_go_minimax_m3_multiturn_clarification_turn1.sse",
            "anthropic/opencode_go_minimax_m3_multiturn_clarification_turn2.sse",
        ])
        .protocol("anthropic")
        .args(["print", "<turn1 prompt>", "<turn2 prompt>"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("<expected substring>"))
        .success();
}
```

Use the prompts and expected substrings from the existing `deepseek_v4_pro`
multi-turn tests as a template. The exact prompts are embedded in the fixture
names: `math_chain`, `weather_chain`, `read_summarize_followup`,
`reasoning_followup`, `multi_tool_then_compare`, `clarification`.

### TUI tests in `tests/tui_replay_conversations.rs`

For each single-turn fixture, append a `#[tokio::test]` function:

```rust
#[tokio::test]
async fn tui_minimax_m2_5_simple_renders() {
    test_tui()
        .fixture("anthropic/opencode_go_minimax_m2_5_simple.sse")
        .protocol("anthropic")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("ok"));
}
```

Repeat for the `tool` and `reasoning` fixtures with appropriate prompts and
assertions.

For each `minimax_m3` multi-turn pair, append a `#[tokio::test]` function that
sends both prompts sequentially:

```rust
#[tokio::test]
async fn tui_minimax_m3_multiturn_weather_chain_renders() {
    test_tui()
        .fixtures([
            "anthropic/opencode_go_minimax_m3_multiturn_weather_chain_turn1.sse",
            "anthropic/opencode_go_minimax_m3_multiturn_weather_chain_turn2.sse",
        ])
        .protocol("anthropic")
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

## Edge cases to assert

- `tool` fixtures invoke the fake `get_weather` registry and emit deterministic output.
- `reasoning` fixtures render a reasoning/thinking block.
- Multi-turn fixtures preserve context across turns.

## Dependencies

- `cli_replay_conversations`
- `tui_replay_conversations`
- `black_box_replay_dsl`

## Acceptance checklist

- [x] All 11 fixtures above are referenced by at least one test.
- [x] `cargo test --test cli_replay` passes for newly added CLI tests.
- [x] `cargo test --test tui_replay_conversations` passes for newly added TUI tests.
- [x] No fixture remains unwired after this task.
