# Actor Runtime Decision: Simple Tokio Tasks vs Framework

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

Runie currently runs three ad-hoc tokio tasks in `runie-term/src/main.rs`
(`agent_loop`, `input_reader`, `render_task`) plus the main `event_loop`.
There is no formal actor boundary: actors share `mpsc` channels directly,
state lives in the event loop, and there is no typed bus or lifecycle.

Research of `~/Code/agents` showed two camps:
- **Lightweight**: Goose, Codex, thClaws, OpenHarness use tokio tasks + typed
  channels/broadcast, no actor framework.
- **Heavyweight**: AutoGen (`SingleThreadedAgentRuntime`), Coerce, Actix use a
  full actor runtime.

Given Runie’s `AGENTS.md` rule to keep it stupidly simple and the prior dead
actor-framework code (`ARCHITECTURE_ROUND2.md` §1.2), we will stay lightweight.
This task makes that decision explicit and defines the minimal actor traits
we actually need.

## Acceptance Criteria

- [ ] ADR `docs/adr/0017-actor-runtime.md` records the decision: **simple tokio
  tasks + typed `Actor` trait + `EventBus`, no external actor framework**.
- [ ] `crates/runie-core/src/actor.rs` defines a minimal trait:
  ```rust
  pub trait Actor: Send + 'static {
      type Msg: Send + Clone;
      type Event: Send + Clone;
      fn run(self, rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) -> impl Future<Output = ()> + Send;
  }
  ```
  (or an equivalent non-async-trait design if edition 2021 requires it).
- [ ] `crates/runie-core/src/bus.rs` defines a typed `EventBus<E>` using
  `tokio::sync::broadcast` with a small replay buffer (last N events) so late
  subscribers (e.g., a reconnected UI) never miss startup events.
- [ ] Documented actor boundaries:
  - `InputActor` — crossterm events → `InputEvent`
  - `AgentActor` — LLM + tools → `AgentEvent`
  - `ConfigActor` — config watcher → `ConfigEvent`
  - `SessionActor` — JSONL persistence ← all durable events
  - `UiActor` — subscribes to bus, owns `AppState` projection, emits snapshots
- [ ] Existing `runie-term/src/main.rs` channel plumbing is **not** rewritten in
  this task; this task only produces the trait + ADR + a small proof-of-concept
  test actor.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_bus_replays_last_n_events` — replay buffer returns the last K events to a new subscriber.
- [ ] `actor_trait_runs_and_receives_messages` — a test actor processes 3 messages and exits cleanly.
- [ ] `actor_supervision_cancels_on_drop` — dropping the actor handle cancels the task.

### Layer 2 — Event Handling
- [ ] `bus_publish_subscribe_round_trip` — publish `AgentEvent::Thinking`, subscriber receives it.

### Layer 3 — Rendering
N/A (no UI change).

## Notes

**Rejected options (document in ADR):**
- `actix` — too heavy, brings its own runtime and web baggage.
- `coerce` — distributed-systems oriented; overkill for a single-machine TUI.
- A custom JSON-serializing actor framework (the dead code from Round 1) —
  type erasure via JSON made debugging painful.

**Why keep it simple:**
- Rust’s `tokio::sync` already gives us everything we need.
- Actor boundaries are enforced by types and channels, not a runtime.
- Easier to test and profile than a framework.

**Files touched:**
- `crates/runie-core/src/actor.rs` (new)
- `crates/runie-core/src/bus.rs` (new)
- `docs/adr/0017-actor-runtime.md` (new)
- `docs/adr/README.md` (update)

**Out of scope:**
- Rewiring `runie-term` to use the new actor trait (covered by `event-bus-jsonl-persistence`).
- Distributed actors, supervisors, or restart strategies.
