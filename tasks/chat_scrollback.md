# Chat scrollback

## Objective

Verify keyboard-driven scrollback focus and navigation.

## Grok behavior observed

- `Tab` focuses scrollback; arrows/PageUp/PageDown scroll; `Esc` returns to input.

## runie current state

runie supports scrollback but keyboard focus behavior needs black-box coverage.

## Required runie changes

- Ensure Tab moves focus to scrollback and Esc returns to input consistently.

## Test scenarios

1. **Tab focuses scrollback**
   - Keys: `press Tab`
   - Assert: `scrollback|focus`

2. **Arrow scrolls**
   - Keys: `press Up Up`
   - Assert: `previous content`

3. **Page keys scroll**
   - Keys: `press PageUp`
   - Assert: `scrollback`

4. **Esc returns to input**
   - Keys: `press Escape`
   - Assert: `❯`

## Edge / negative cases

- Scrollback focus does not swallow input keys after Esc.
- Empty history scrolls without panic.

## Dependencies

- `turn_lifecycle`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
