# P56 — Async input dispatch latency

Status: complete (2026-08-08)

## Finding

The terminal reader and input mailbox were asynchronous, but the application
loop used `Interval::reset()` after receiving each key. Tokio interprets that
as “schedule the next tick one full period from now”, so the 50 ms render
cadence became a per-key debounce. Fast prompts could therefore accumulate
visible input delay.

## Change

Use `Interval::reset_immediately()` after enqueueing input. The existing
single-owner input actor and FIFO pending-key queue remain unchanged; the
dispatch wake-up now occurs immediately without blocking the async loop or
mutating another actor's state.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p runie-tui --bin runie`
- `just ci`

The protected user files `AGENTS.md` and
`crates/runie-tui/tests/e2e/visual-activity-mixed.yaml` were not modified.
