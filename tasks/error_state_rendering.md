# Error state rendering

## Objective

Verify that runie CLI and TUI render provider errors, invalid API keys, and
replay error fixtures gracefully without crashing or leaking secrets.

## Why this matters

Black-box tests must cover failure paths as thoroughly as happy paths. Errors
are a normal part of the provider lifecycle and are surfaced to the user
clearly.

## Error sources to cover

1. **Invalid API key** — config has a provider with a bogus key; validation fails
   locally without a network call.
2. **Provider error fixture** — recorded `error:` SSE lines or HTTP-status
   replay fixtures drive the provider.
3. **Missing replay fixture file** — `RUNIE_REPLAY_FIXTURES` points to a path
   that does not exist.
4. **Standalone network/auth errors against a live provider** — out of scope for
   the deterministic black-box suite; covered indirectly by invalid-key and
   replay-error scenarios.

## Existing fixtures

All fixtures live under `runie-tests/fixtures/`:

- `runie-tests/fixtures/{openai,anthropic}/opencode_go_*.sse` (happy-path; use
  for contrast)
- Error fixtures added by:
  - `tasks/add-openai-error-sse-fixtures.md`
  - `tasks/add-anthropic-error-sse-fixtures.md`
  - `tasks/add-http-status-error-replay-support-and-tests.md`

## Required runie changes

- Ensure `runie-cli print` exits non-zero and emits an error event when the
  provider returns an error.
- Ensure `runie-tui` shows an error banner or status-bar error state instead of
  hanging.
- Ensure no API key value is printed in error output.

## Test scenarios

1. **CLI invalid API key via replay fixture**
   - Setup: `RUNIE_REPLAY_FIXTURES=runie-tests/fixtures/openai/invalid_api_key.sse`.
   - Args: `runie print "hello"`
   - Assert: exit code is non-zero; stderr contains `Invalid|error|failed|Missing API key`;
     no key value in stdout/stderr.

2. **CLI rate-limit replay fixture**
   - Setup: `RUNIE_REPLAY_FIXTURES=runie-tests/fixtures/openai/rate_limit_error.sse`.
   - Args: `runie print "hello"`
   - Assert: output contains `error` or `rate limit`; process exits cleanly.

3. **TUI invalid API key via replay fixture**
   - Setup: `AppTest::with_config(TestConfig::mock())` plus
     `RUNIE_REPLAY_FIXTURES=runie-tests/fixtures/openai/invalid_api_key.sse`.
   - Keys: `type 'hi' press Enter wait_for_idle`
   - Assert: pane contains `error|failed|invalid` and does not contain the key.

4. **TUI missing replay fixture**
   - Setup: `RUNIE_REPLAY_FIXTURES=/nonexistent/file.sse`.
   - Keys: `type 'hi' press Enter wait_for_idle`
   - Assert: pane contains `error|failed|not found`; app remains interactive.

5. **Onboarding invalid key**
   - Setup: `AppTest::onboarding()` and attempt to add a provider with key `sk-bad`.
   - Keys: `select provider type bad key press Enter`
   - Assert: `Invalid|error|failed`; dialog stays open.

## Edge / negative cases

- Error output strips ANSI escape codes in assertions but preserves them in UI.
- Long error messages wrap without panic.
- Rapid error/submit cycles do not corrupt the input box.

## Dependencies

- `black_box_replay_testing`
- `core_mock_loop`
- `startup_onboarding`
- `add_openai_error_sse_fixtures`
- `add_anthropic_error_sse_fixtures`
- `wire_error_sse_fixtures_to_cli_replay_tests`
- `wire_error_sse_fixtures_to_tui_replay_tests`
- `add_http_status_error_replay_support_and_tests`
- `add_error_recovery_and_retry_blackbox_tests`

## Acceptance checklist

- [x] All scenarios pass in CLI and TUI where applicable.
- [x] Each test uses a temp `$HOME` so config is isolated.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
- [x] No secret values appear in assertion output or captured pane text.
