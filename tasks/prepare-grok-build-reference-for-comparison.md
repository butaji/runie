# Prepare Grok Build reference for comparison

**Status**: todo
**Milestone**: R7
**Category**: Testing
**Priority**: P0

**Depends on**: none
**Blocks**: build-runie-vs-grok-build-comparison-harness, compare-headless-one-shot-scenarios-and-fix-gaps

## Description

The installed Grok Build binary (`/opt/homebrew/bin/grok`) is currently killed by macOS quarantine (`com.apple.quarantine` attribute). Before Runie can be compared against it, the binary must be made runnable and authenticated.

## Acceptance Criteria

- [ ] Remove the macOS quarantine attribute from the Grok Build binary.
- [ ] `grok --version` or `grok --help` runs without `Killed: 9`.
- [ ] Authentication succeeds via browser OAuth or `XAI_API_KEY`.
- [ ] A simple headless command (`grok --no-auto-update -p "hello"`) returns output and exits.
- [ ] A simple TUI launch under tmux reaches a usable prompt.
- [ ] Any authentication failure is documented as a blocker task.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `grok_build_headless_hello` — run `grok --no-auto-update -p "hello"` and assert exit code 0 with non-empty stdout.
- [ ] `grok_build_tui_launches` — run `grok` in tmux, capture pane, and assert a welcome/prompt string appears.

## Files touched

- No code changes; system attribute on `/opt/homebrew/Caskroom/grok-build/0.2.72/grok-0.2.72-macos-aarch64`.
- `scripts/compare-with-grok-build.sh` (created in follow-up task).

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- If authentication is unavailable, this task becomes a documented blocker and the comparison proceeds using published Grok Build documentation as a secondary reference.
- Do not run Grok Build inside the `runie-dev` working directory; always use a temp copy.
