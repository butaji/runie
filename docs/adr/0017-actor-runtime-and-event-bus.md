# Actor Runtime and Event Bus with JSONL Persistence

## Context

Runie grew from a single event loop with ad-hoc `mpsc` channels into a system
where agent work, input handling, config watching, rendering, and session
persistence are all coupled through `AppState`. `docs/ARCHITECTURE_ROUND2.md`
proposed a pipe-based redesign to fix this, but it was never implemented.

Research of `~/Code/agents` (Goose, Codex, OpenHarness, thClaws, Kimi Code,
Gemini CLI) shows a clear convergence: keep the runtime lightweight with tokio
tasks and typed channels/broadcast, persist sessions as append-only event logs,
and let UI be a projection of the event stream.

## Decision

1. **No external actor framework.** Runie will use simple tokio tasks plus a
   minimal `Actor` trait. We explicitly reject Actix, Coerce, and the prior
   JSON-serializing actor framework.
2. **Typed event bus.** All cross-actor communication goes through
   `EventBus<CoreEvent>` built on `tokio::sync::broadcast` with a bounded replay
   buffer.
3. **Event-sourced sessions.** Durable events are appended to JSONL files under
   `data_dir/runie/sessions/<id>.jsonl`. Sessions are resumed by replaying those
   events. No SQLite.
4. **State lives in actors/projection actors.** `AppState` becomes the owned
   projection of the UI actor, not the global god object.

## Consequences

- **Positive:** Clear boundaries, testable actors, hot-reconnecting UI via replay.
- **Positive:** Human-readable session files; no database dependency.
- **Trade-off:** More boilerplate than a framework, but far less than the prior
  custom actor framework.
- **Trade-off:** JSONL is not as query-friendly as SQLite; session summaries and
  indexes are maintained separately.
