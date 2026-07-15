# Input composition

## Objective

Verify that the chat input box handles text, multiline, special characters, and cursor editing.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Grok bottom input shows `❯` placeholder.
- Supports multiline via `Ctrl+M` toggle and `Ctrl+J`; `Enter` sends.

## runie current state

runie supports typing, `Shift+Enter` (tmux F3) for newline, `Ctrl+J` for newline, and basic cursor/editing keys.

## Required runie changes

- No input-box change required; ensure both `Shift+Enter` and `Ctrl+J` insert newlines.

## Test scenarios

1. **Type simple text**
   - Keys: `type `hello``
   - Assert: `❯ hello`

2. **Submit with Enter**
   - Keys: `press Enter`
   - Assert: `hello`

3. **Multiline Shift+Enter**
   - Keys: `type `line1` press F3 type `line2` press Enter`
   - Assert: `line1.*line2`

4. **Multiline Ctrl+J**
   - Keys: `type `a` press C-j type `b` press Enter`
   - Assert: `a.*b`

5. **Unicode input**
   - Keys: `type `привет 🌍``
   - Assert: `привет`

6. **Empty submit**
   - Keys: `press Enter`
   - Assert: `mock/echo`

7. **Cursor start/end**
   - Keys: `type `abc` press C-a then C-e`
   - Assert: `abc`

8. **Delete word**
   - Keys: `type `hello world` press C-w`
   - Assert: `hello`

## Edge / negative cases

- Very long input wraps without crashing.
- Special regex chars in input are escaped in assertions.

## Dependencies

- None

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
