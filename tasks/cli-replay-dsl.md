# Unify CLI replay tests around the DSL

## Objective

Rewrite `tests/cli_replay.rs` and `tests/error_state_rendering.rs` CLI
paths to use a fluent `test_cli()` DSL instead of manually spawning processes,
creating temp homes, and setting env vars.

## Why this matters

Manual process spawning duplicates isolation logic already implemented in the
DSL, makes tests longer, and does not validate that the DSL works for CLI
tests. A unified DSL also makes it trivial to add new fixture-based CLI tests.

## Proposed DSL

```rust
test_cli()
    .fixture("openai/opencode_go_deepseek_v4_flash_simple.sse")
    .args(["print", "say ok"])
    .assert()
    .stdout_contains("ok")
    .success();
```

The builder must:

- Build/find the `runie` CLI binary.
- Create a temp `$HOME` and write a minimal config.
- Set `RUNIE_REPLAY_FIXTURES`, `RUNIE_REPLAY_PROTOCOL`, and any other env vars.
- Run the command and capture stdout/stderr/exit code.

## Files to update

- `tests/cli_replay.rs`
- `tests/error_state_rendering.rs` CLI tests

## Dependencies

- `black_box_replay_dsl`

## Acceptance checklist

- [x] `tests/cli_replay.rs` uses `test_cli()` exclusively.
- [x] `tests/error_state_rendering.rs` CLI tests use `test_cli()`.
- [x] No temp-home or config-writing code remains in `tests/cli_replay.rs`.
- [x] All CLI replay tests in `tests/cli_replay.rs` pass.
