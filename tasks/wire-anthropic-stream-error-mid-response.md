# Wire Anthropic stream_error_mid_response fixture

## Objective

Add the one missing black-box test that exercises
`fixtures/anthropic/stream_error_mid_response.sse` through `runie-cli`, closing
the last recorded-trace coverage gap.

## Why this matters

`docs/fixture-coverage.md` reports that 130 of 131 recorded SSE fixtures are
wired to tests. The single unwired fixture is
`anthropic/stream_error_mid_response.sse`. Until it is referenced by a test, the
suite cannot claim full recorded-trace coverage.

## Required implementation

1. Open `tests/cli_replay.rs`.
2. Locate the existing OpenAI equivalent test
   `openai_stream_error_mid_response_renders` (around line 1034).
3. Immediately after it, add the Anthropic mirror test:

   ```rust
   /// Anthropic stream error mid-response produces a non-zero exit.
   #[tokio::test]
   async fn anthropic_stream_error_mid_response_renders() {
       test_cli()
           .fixture("anthropic/stream_error_mid_response.sse")
           .args(["print", "hello"])
           .assert()
           .await
           .unwrap()
           .failure()
           .stdout(or(contains("error"), contains("server")));
   }
   ```

4. Run the focused test:

   ```bash
   cargo test --test cli_replay anthropic_stream_error_mid_response_renders
   ```

5. Regenerate the coverage report and confirm zero unwired fixtures:

   ```bash
   cargo run --bin fixture-coverage
   ```

   The report header must read:
   **Total Fixtures:** 131 | **Wired:** 131 | **Unwired:** 0.

## Dependencies

- `black_box_replay_testing` (done)
- `cli_replay_dsl` (done)
- `add_anthropic_error_sse_fixtures` (done)

## Acceptance checklist

- [x] `tests/cli_replay.rs` contains `anthropic_stream_error_mid_response_renders`.
- [x] The test references exactly `anthropic/stream_error_mid_response.sse`.
- [x] The test asserts a non-zero exit code and a readable error indication.
- [x] `cargo test --test cli_replay anthropic_stream_error_mid_response_renders` passes.
- [x] `cargo run --bin fixture-coverage` reports 0 unwired fixtures.
