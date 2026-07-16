# Command palette navigation

## Objective

Verify that the command palette opens, filters, navigates, and activates commands with keyboard only.

## Grok behavior observed

- Grok: `Ctrl+P` / `?` opens a categorized command palette.
- Typing filters commands; arrow keys / Tab move; Enter selects; Esc closes.

## runie current state

runie already has a `Ctrl+P` palette with categories (System, Session, Core, Model, Safety) and supports filtering.

## Required runie changes

- No behavior change required; just comprehensive black-box coverage.

## Test scenarios

1. **Open with Ctrl+P**
   - Keys: `press C-p`
   - Assert: `Commands|↑↓ navigate`

2. **Open with slash**
   - Keys: `type `/``
   - Assert: `Commands`

3. **Filter to model**
   - Keys: `type `model``
   - Assert: `Switch model|/model`

4. **Down selects next**
   - Keys: `press Down`
   - Assert: `▸`

5. **Tab selects next**
   - Keys: `press Tab`
   - Assert: `▸`

6. **Shift+Tab selects previous**
   - Keys: `press Down Down S-Tab`
   - Assert: `▸`

7. **Enter runs command**
   - Keys: `press Enter`
   - Assert: `Select Model`

8. **Esc closes palette**
   - Keys: `press Escape`
   - Assert: `Type a message|mock/echo`

## Edge / negative cases

- No-match filter shows 'No matching commands'.
- Palette closes on Ctrl+Q without crashing.

## Dependencies

- `keyboard_shortcuts_overlay`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
