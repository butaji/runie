# Convert TUI replay tests to real fixtures

## Objective

Rewrite `tests/tui_replay_conversations.rs` so it is actually driven by recorded
SSE fixtures instead of mock echo responses.

## Why this matters

The file name and comments reference OpenAI/Anthropic replay fixtures, but the
implementation currently uses `AppTest::mock()` and asserts echo responses. This
gives false confidence and leaves realistic provider rendering (streaming,
tool blocks, thinking blocks) untested.

## Required changes

1. Replace `AppTest::mock()` setup in each test with a fixture-driven setup that
   sets `RUNIE_REPLAY_FIXTURES` and the protocol hint.
2. Use the fixture matrix defined in `tasks/tui_replay_conversations.md`.
3. Assert on realistic output (tool block headings, thinking indicator, final
   answers) rather than echo text.
4. Use the static fake tool registry for tool-call fixtures.

## Fixtures to exercise

All fixtures listed in `tasks/tui_replay_conversations.md` must be covered.

## Dependencies

- `black_box_replay_dsl`
- `tui_dsl_polling_waits`
- `dsl_permission_dialog_helpers`

## Acceptance checklist

- [x] Every test in `tests/tui_replay_conversations.rs` references a real
      fixture path.
- [x] No test asserts echo responses.
- [x] All TUI replay scenarios pass without API keys.
- [x] File is renamed if it no longer contains mock tests.
