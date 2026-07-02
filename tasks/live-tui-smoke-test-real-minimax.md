# Live TUI smoke test with real MiniMax provider

**Status**: wontfix
**Milestone**: R7
**Category**: Testing
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

This task was blocked on having a real `MINIMAX_API_KEY`. The task requires:
1. An actual MiniMax API key (env var or keychain)
2. A configured `~/.runie/config.toml` with the `minimax` provider

Since we don't have access to a real MiniMax API key in this environment, this task cannot be completed. The mock-provider smoke tests (`scripts/tmux-smoke-test.sh mock`) already pass.

## Why Wontfix

- Requires a real API key that is not available in this environment
- Cannot be automated without network access and cost
- Mock tests already validate the TUI behavior
- Real API testing should be done manually when an API key is available

## Files touched

None - no changes were needed

## Validation

- Mock provider tests pass: `scripts/tmux-smoke-test.sh mock`
- TUI functionality verified via other tests
