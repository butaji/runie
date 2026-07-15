# Provider management

## Objective

Verify provider add/disconnect and API-key validation flows.

## Grok behavior observed

- Provider/commands for adding accounts and choosing models.

## runie current state

runie has `/provider` and onboarding provider picker.

## Required runie changes

- Ensure invalid API key is handled gracefully in black-box tests.

## Test scenarios

1. **Open provider dialog**
   - Keys: `type `/provider` press Enter`
   - Assert: `Providers`

2. **Navigate actions**
   - Keys: `press Tab`
   - Assert: `▸`

3. **Add mock provider**
   - Keys: `select mock enter key`
   - Assert: `mock`

4. **Invalid key**
   - Keys: `enter bad key`
   - Assert: `Invalid|error|failed`

5. **Disconnect provider**
   - Keys: `select disconnect confirm`
   - Assert: `disconnected`

## Edge / negative cases

- Disconnecting last provider re-opens onboarding.
- Cancel keeps existing provider.

## Dependencies

- `startup_onboarding`
- `settings_dialog`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
