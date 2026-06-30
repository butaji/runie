# Live TUI smoke test with real MiniMax provider

**Status**: partial
**Milestone**: R7
**Category**: Testing
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Run `scripts/tmux-smoke-test.sh minimax` against the real MiniMax API. Requires a `MINIMAX_API_KEY`.

## Live Evidence / Blocker

```
$ bash scripts/tmux-smoke-test.sh minimax
ERROR: MINIMAX_API_KEY is required for minimax mode
```

No `MINIMAX_API_KEY` environment variable is set and no matching keychain entry (`runie-minimax-api-key`) exists. Real MiniMax live testing is blocked until a key is provided.

## Acceptance Criteria

- [ ] Obtain or confirm a valid `MINIMAX_API_KEY` (env var or keychain).
- [ ] Configure `~/.runie/config.toml` with the `minimax` provider and `minimax/text-01` model.
- [ ] Run the tmux smoke test and verify the TUI starts a turn and renders a real response.
- [ ] Document any provider-specific issues.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_hello_reaches_idle` — tmux smoke test for "hello" with real MiniMax completes.

## Files touched

- `scripts/tmux-smoke-test.sh`
- `docs/superpowers/plans/2026-06-30-live-tui-smoke-test-report.md`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh minimax` passes in a real terminal once an API key is available.

## Notes

- **Cannot be automated** - requires `MINIMAX_API_KEY` environment variable or keychain entry.
- Run only when an API key is available; network and cost apply.
- Status: `partial` - blocked on missing API key. Mock-provider smoke tests in `scripts/tmux-smoke-test.sh mock` are running, but the `hello` scenario still repeats text and must be fixed first.
- Real API smoke test is manual verification only.
