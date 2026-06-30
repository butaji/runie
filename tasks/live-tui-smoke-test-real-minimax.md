# Live TUI smoke test with real MiniMax provider

**Status**: todo
**Milestone**: R7
**Category**: Testing
**Priority**: P1

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Run `scripts/tmux-smoke-test.sh minimax` against the real MiniMax API. Requires a `MINIMAX_API_KEY`.

## Acceptance Criteria

- [ ] Obtain or confirm a valid `MINIMAX_API_KEY`.
- [ ] Configure `~/.runie/config.toml` with the `minimax` provider and `minimax/text-01` model.
- [ ] Run the tmux smoke test and verify the TUI starts a turn and renders a real response.
- [ ] Document any provider-specific issues.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_hello_reaches_idle` — tmux smoke test for "hello" with real MiniMax completes.

## Files touched

- `scripts/tmux-smoke-test.sh`
- `docs/superpowers/plans/2026-06-30-live-tui-smoke-test-report.md`

## Notes

- Run only when an API key is available; network and cost apply.
