# DSL response assertions

## Objective

Move the duplicated `response_pattern()` helper into the DSL or a shared
predicate module.

## Why this matters

Four test files define the same helper:

```rust
fn response_pattern(msg: &str) -> String {
    format!(r"→\s*{}", regex::escape(msg))
}
```

Centralizing it removes duplication and gives tests a common vocabulary for
asserting that the assistant echoed or responded to a prompt.

## Proposed API

Implemented in `src/app_test.rs`:

```rust
// TUI
app.expect_response("hello").await?;
```

For CLI replay tests, continue using `assert.stdout(contains("hello"))` from the
predicate module. The helper escapes the message and matches the rendered
response arrow (`→`).

## Files with duplication

- `tests/mock_echo.rs:19`
- `tests/mock_list_files.rs:16`
- `tests/core_mock_loop.rs:21`
- `tests/turn_lifecycle.rs` (inline)

## Dependencies

- `black_box_replay_dsl`

## Acceptance checklist

- [x] Shared helper exists in the DSL or `predicates` module.
- [x] Duplicated helpers are removed from test files.
- [x] Tests still pass.
