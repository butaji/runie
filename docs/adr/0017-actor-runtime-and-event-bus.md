# Actor Runtime and Event Bus with JSONL Persistence

## Context

Runie grew from a single event loop with ad-hoc `mpsc` channels into a system
where agent work, input handling, config watching, rendering, and session
persistence are all coupled through `AppState`. An earlier pipe-based
redesign was proposed to fix this but was never implemented.

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

## Implementation

### Actor Trait (`crates/runie-core/src/actor.rs`)

```rust
pub trait Actor: Send + 'static {
    type Msg: Send + Clone + 'static;
    type Event: Send + Clone + 'static;
    fn run(self, rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) -> impl Future<Output = ()> + Send;
}
```

### EventBus (`crates/runie-core/src/bus.rs`)

```rust
pub struct EventBus<E: Send + Clone + 'static> {
    sender: tokio::sync::broadcast::Sender<E>,
    replay: Arc<ReplayBuffer<E>>,
}

impl<E: Send + Clone + 'static> EventBus<E> {
    pub fn new(capacity: usize) -> Self;
    pub fn publish(&self, event: E) -> usize;
    pub fn subscribe(&self) -> broadcast::Receiver<E>;
    pub fn subscribe_with_replay(&self) -> ReplayReceiver<E>;
}
```

### Actor Boundaries (documented)

| Actor | Input | Output |
|-------|-------|--------|
| `InputActor` | crossterm events | `InputEvent` |
| `AgentActor` | LLM + tools | `AgentEvent` |
| `ConfigActor` | config watcher | `ConfigEvent` |
| `SessionActor` | all durable events | JSONL persistence |
| `UiActor` | subscribes to bus | owns `AppState` projection |

## Rejected Alternatives

| Framework | Reason for Rejection |
|-----------|---------------------|
| Actix | Too heavy, web-oriented, brings own runtime |
| Coerce | Distributed-systems oriented, overkill for TUI |
| Xactor | Unmaintained |
| Custom JSON framework | Prior dead code made debugging painful via type erasure |

## Consequences

- **Positive:** Clear boundaries, testable actors, hot-reconnecting UI via replay.
- **Positive:** Human-readable session files; no database dependency.
- **Positive:** Uses tokio we already depend on; zero new dependencies.
- **Trade-off:** More boilerplate than a framework, but far less than the prior
  custom actor framework.
- **Trade-off:** JSONL is not as query-friendly as SQLite; session summaries and
  indexes are maintained separately.
- **Trade-off:** No built-in supervision/restart strategies; can be added manually.
