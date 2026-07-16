# Wire remaining OpenAI Mimo fixtures

## Objective

Add black-box test coverage for the remaining OpenAI-protocol Mimo fixtures that are on disk but not yet referenced by any test.

## Fixtures to wire

```text
fixtures/openai/opencode_go_mimo_v2_5_multi_tool.sse
fixtures/openai/opencode_go_mimo_v2_5_pro_simple.sse
fixtures/openai/opencode_go_mimo_v2_5_pro_tool.sse
```

(`fixtures/openai/opencode_go_mimo_v2_5_simple.sse`,
`fixtures/openai/opencode_go_mimo_v2_5_tool.sse`, and
`fixtures/openai/opencode_go_mimo_v2_5_reasoning.sse` are already wired.)

## Implementation

Add tests to `tests/cli_replay.rs` and `tests/tui_replay_conversations.rs`. Do not create new test files.

### CLI tests in `tests/cli_replay.rs`

Append three `#[tokio::test]` functions:

```rust
#[tokio::test]
async fn mimo_v2_5_multi_tool_replays() {
    test_cli()
        .fixture("openai/opencode_go_mimo_v2_5_multi_tool.sse")
        .args(["print", "weather in Paris and Berlin"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("get_weather"))
        .success();
}

#[tokio::test]
async fn mimo_v2_5_pro_simple_replays() {
    test_cli()
        .fixture("openai/opencode_go_mimo_v2_5_pro_simple.sse")
        .args(["print", "say ok"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("ok"))
        .success();
}

#[tokio::test]
async fn mimo_v2_5_pro_tool_replays() {
    test_cli()
        .fixture("openai/opencode_go_mimo_v2_5_pro_tool.sse")
        .args(["print", "weather in Paris"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("get_weather"))
        .success();
}
```

### TUI tests in `tests/tui_replay_conversations.rs`

Append matching `#[tokio::test]` functions using
`test_tui().fixture(...).type_keys(...).submit().wait_for_idle(...).capture_pane().assert(...)`.

## Dependencies

- `cli_replay_conversations`
- `tui_replay_conversations`
- `black_box_replay_dsl`

## Acceptance checklist

- [x] All 3 unwired fixtures above are referenced by at least one test.
- [x] `cargo test --test cli_replay` passes for newly added CLI tests.
- [x] `cargo test --test tui_replay_conversations` passes for newly added TUI tests.
- [x] No fixture remains unwired after this task.
