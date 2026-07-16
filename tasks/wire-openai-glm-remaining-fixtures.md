# Wire remaining OpenAI GLM fixtures

## Objective

Add black-box test coverage for the remaining OpenAI-protocol GLM fixtures that are on disk but not yet referenced by any test.

## Fixtures to wire

### Single-turn

```text
fixtures/openai/opencode_go_glm_5_simple.sse
fixtures/openai/opencode_go_glm_5_tool.sse
fixtures/openai/opencode_go_glm_5_1_simple.sse
fixtures/openai/opencode_go_glm_5_2_simple.sse
fixtures/openai/opencode_go_glm_5_2_tool.sse
fixtures/openai/opencode_go_glm_5_2_reasoning.sse
```

### Multi-turn (`glm_5_2`)

```text
fixtures/openai/opencode_go_glm_5_2_multiturn_math_chain_turn1.sse
fixtures/openai/opencode_go_glm_5_2_multiturn_math_chain_turn2.sse
fixtures/openai/opencode_go_glm_5_2_multiturn_reasoning_followup_turn1.sse
fixtures/openai/opencode_go_glm_5_2_multiturn_reasoning_followup_turn2.sse
fixtures/openai/opencode_go_glm_5_2_multiturn_weather_chain_turn1.sse
fixtures/openai/opencode_go_glm_5_2_multiturn_weather_chain_turn2.sse
```

## Implementation

Add tests to `tests/cli_replay.rs` and `tests/tui_replay_conversations.rs`. Do not create new test files.

### CLI tests in `tests/cli_replay.rs`

For each single-turn fixture, append a `#[tokio::test]` function using
`test_cli().fixture(...).args(["print", ...]).assert()`.

For each `glm_5_2` multi-turn pair, append a `#[tokio::test]` function using
`.fixtures([turn1, turn2])` and the matching two-prompt `args(["print", ...])`.
Use the existing `deepseek_v4_pro` multi-turn tests as the template for prompts
and assertions.

### TUI tests in `tests/tui_replay_conversations.rs`

For each single-turn fixture, append a `#[tokio::test]` function using
`test_tui().fixture(...).type_keys(...).submit().wait_for_idle(...).capture_pane().assert(...)`.

For each `glm_5_2` multi-turn pair, append a `#[tokio::test]` function that
sends both prompts sequentially with `.fixtures([turn1, turn2])`.

## Dependencies

- `cli_replay_conversations`
- `tui_replay_conversations`
- `black_box_replay_dsl`

## Acceptance checklist

- [x] All 12 fixtures above are referenced by at least one test.
- [x] `cargo test --test cli_replay` passes for newly added CLI tests.
- [x] `cargo test --test tui_replay_conversations` passes for newly added TUI tests.
- [x] No fixture remains unwired after this task.
