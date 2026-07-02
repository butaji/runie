# Centralize agent approval timeout and max tool rounds

## Status

`todo`

## Description

`runie-agent/src/actor/handlers.rs:65` hardcodes `60` seconds for permission-request timeout, duplicating `EmitApprovalSink::new()`. `runie-agent/src/actor/mod.rs:215` hardcodes `5` max tool rounds, duplicating `HeadlessCliOptions`.

## Acceptance criteria

1. **Unit tests** — Both values are named constants used by all call sites.
2. **E2E tests** — Permission timeout and tool-round limits behave as before in replay.
3. **Live tmux tests** — Run a multi-tool turn in tmux and verify the timeout/round limits still apply.

## Tests

### Unit tests
- Constants exist and are referenced.

### E2E tests
- Replay exercises permission timeout and max tool rounds.

### Live tmux tests
- Trigger a permission prompt and a tool loop in tmux.
