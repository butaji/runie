# File references

## Objective

Verify `@` file references attach files to the prompt.

## Grok behavior observed

- Tip: `Use @ to attach files like @src/main.rs`.

## runie current state

runie has `@` refs support in core; black-box coverage is light.

## Required runie changes

- Add tests for @ completion/hints; @! hidden files out of scope.
- Tests must create target files inside the isolated temp workspace (under the
  temp `$HOME`) rather than reading arbitrary host filesystem paths.

## Test scenarios

1. **Type @**
   - Keys: `type `@``
   - Assert: `@|file|path`

2. **Attach file**
   - Keys: `type `@Cargo.toml` press Enter`
   - Assert: `Cargo\.toml`

3. **Submit with ref**
   - Keys: `type ` explain` press Enter`
   - Assert: `explain`

## Edge / negative cases

- Invalid path handled gracefully.
- Multiple @ refs in one prompt work.

## Dependencies

- `input_composition`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
