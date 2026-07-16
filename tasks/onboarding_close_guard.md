# Onboarding close guard

## Objective

Verify that the first-run onboarding provider picker is blocking: it cannot be
closed with Esc, Cancel, or Abort until the user has connected a provider and
selected at least one model.

## Why this matters

This is a core app-invariant documented in `AGENTS.md`. Without coverage, a
regression can let users reach the chat UI with no configured provider.

## runie current state

runie shows a provider picker on first start. Esc and the `/quit` text command
must not close it until onboarding is complete.

## Required runie changes

- No behavior change required; ensure black-box coverage is comprehensive and
  explicit.

## Test scenarios

1. **Esc does not close onboarding**
   - Setup: `AppTest::onboarding() start`.
   - Keys: `press Escape`
   - Assert: pane still contains `Choose a provider|Select a provider`.

2. **Ctrl+Q quits from idle onboarding**
   - Setup: `AppTest::onboarding() start`.
   - Keys: `press Ctrl+Q`
   - Assert: process exits cleanly.

3. **Text quit commands are blocked**
   - Setup: `AppTest::onboarding() start`.
   - Keys: `type 'quit' press Enter`
   - Assert: onboarding dialog remains; pane contains `Choose a provider`.

4. **Complete onboarding unlocks UI**
   - Setup: `AppTest::onboarding() start`.
   - Keys: `select mock provider enter key select echo model press Enter`
   - Assert: pane contains `mock/echo|Type a message` and onboarding is gone.

5. **Onboarding with mock provider visible**
   - Setup: `AppTest::mock_onboarding() start`.
   - Keys: `select mock press Enter select echo press Enter`
   - Assert: chat UI appears with `mock/echo`.

## Edge / negative cases

- Disconnecting the last connected provider re-opens onboarding.
- Invalid config during onboarding shows validation and stays in onboarding.

## Dependencies

- `startup_onboarding`

## Acceptance checklist

- [x] All scenarios pass with `AppTest::onboarding()` or `AppTest::mock_onboarding()`.
- [x] Each test uses a temp `$HOME`.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
