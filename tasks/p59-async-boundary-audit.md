# P59 — Async boundary audit

Status: audited (2026-08-08; runtime sync helper quarantined)

## Evidence

The production actor/provider paths were scanned after P57 and P58:

- prompt file search/preview uses `tokio::fs` inside `PromptActor`
- replay SSE/provider fixture loading uses `tokio::fs`
- session persistence already uses `tokio::fs` behind `SessionStorageActor`
- provider retry delays and cancellation use Tokio timers/signals
- all production spawned workers are attached to actor/task owners or joined

The former synchronous `PromptWidget::open_file_search` helper is now compiled
only for unit tests. The live `PromptActor` path can only call the async
Tokio-filesystem implementation, so a blocking file search cannot be reached
from the runtime TUI.

The remaining `std::sync::Mutex<Vec<AgentEvent>>` is an explicitly documented
compatibility-only side buffer used when no event bus is supplied. Live tool
execution uses the event bus and does not use that buffer. Synchronous reads in
`cast_compare`, the TUI fixture loader, and the standalone capture/e2e binary
are offline command-line tooling, not runtime actor paths.

## Rule

Any new runtime filesystem or transport operation must enter an owned actor or
provider future and use Tokio I/O. Compatibility adapters must remain visibly
quarantined and must not become a production delivery path.

## Verification

The full `just ci` gate passes after the P57/P58 changes, including actor,
replay, YAML, visual, and boundary validators.
