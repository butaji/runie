# P56 — Async input dispatch latency

Status: complete (2026-08-08)

## Finding

The terminal reader and input mailbox were asynchronous, but the application
loop used `Interval::reset()` after receiving each key. Tokio interprets that
as “schedule the next tick one full period from now”, so the 50 ms render
cadence became a per-key debounce. Fast prompts could therefore accumulate
visible input delay.

## Change

Use `Interval::reset_immediately()` after enqueueing input and drain the
single-owner FIFO in one async dispatch turn. The dispatch path no longer
spaces characters across render ticks: each key is still reduced by the owning
prompt/UI actor in order, and rendering only observes the resulting snapshot.
This removes the last render-cadence dependency from burst input while keeping
the terminal reader and actor mailboxes asynchronous.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p runie-tui --bin runie`
- burst dispatch drains `Hey` in one actor turn without a render-tick delay
- `just ci`

The protected user files `AGENTS.md` and
`crates/runie-tui/tests/e2e/visual-activity-mixed.yaml` were not modified.
