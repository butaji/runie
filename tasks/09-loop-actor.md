# Step 09: LoopActor + driver

**Status:** pending
**Depends on:** 08

## Goal
Implement the public `LoopActor` API and the pure async `driver` function that runs the agent loop, matching the TS event sequence and barrier rules exactly.

## Changes
- `crates/runie-core/src/loop/turn.rs`:
  - `TurnPlan` enum (Continue, Stop { reason }, ToolBatch { calls }).
  - `decide_next_turn(snapshot, queue_snapshots, hooks) -> TurnPlan` — pure fn, no IO.
- `crates/runie-core/src/loop/driver.rs`:
  - `async fn run_loop(prompts, context, config, signal, deps: LoopDeps) -> Vec<AgentMessage>`:
    1. `agent_start` (await dispatch).
    2. `turn_start` (await dispatch).
    3. For each prompt: `message_start`, `message_end` (await dispatch).
    4. Append prompts to state actor's messages.
    5. Loop until `TurnPlan::Stop`:
       a. `transform_context` (if set).
       b. `convert_to_llm`.
       c. `provider_actor.start(...)` → drain stream events as `message_update` + `message_start`/`message_end` for assistant.
       d. If assistant requested tools: dispatch to `tool_executor_actor` per mode, emit `tool_execution_start/update/end`, then `message_start`/`message_end` for each toolResult in source order.
       e. `should_stop_after_turn` check → emit `turn_end`, exit if true.
       f. Drain steering queue (per `steering_mode`) — inject before next turn.
       g. Else drain follow-up queue (per `follow_up_mode`).
       h. If both empty → emit `turn_end` and exit.
       i. Else `turn_start` for next turn.
    6. `agent_end { messages: <new> }` (await dispatch — barrier).
- `crates/runie-core/src/loop/actor.rs`:
  - `LoopActor` struct holding handles to all child actors, event bus, `JoinHandle<()>` for the current run, abort flag, `SubscriberRegistry`.
  - `LoopDeps { state, steering, follow_up, tool_executor, provider, bus, subscribers }`.
  - Public API:
    ```rust
    impl LoopActor {
        pub fn new(deps: LoopDeps) -> Self;
        pub async fn prompt(&self, msgs: Vec<AgentMessage>) -> Result<Vec<AgentMessage>, LoopError>;
        pub async fn continue_run(&self) -> Result<Vec<AgentMessage>, LoopError>;
        pub async fn steer(&self, msg: AgentMessage);
        pub async fn follow_up(&self, msg: AgentMessage);
        pub fn abort(&self);
        pub async fn wait_for_idle(&self);
        pub fn subscribe(&self, sub: impl Subscriber + 'static) -> SubId;
        pub fn state(&self) -> AgentStateSnapshot;
    }
    ```
- `crates/runie-core/src/loop/mod.rs`: re-exports + `LoopError` enum (Aborted, Internal, StreamError).

## Verification
- `cargo check -p runie-core` → exit 0.
- Behavioural tests in step 12 verify the event sequence; a smoke test here: prompt with a no-tool `MockStreamFn` and assert `agent_start` → `turn_start` → user `message_start/end` → assistant `message_start/update*/end` → `turn_end` → `agent_end`.

## Notes
- The driver is a pure `async fn` — no actor state. The actor owns the `JoinHandle`.
- `terminate: true` is honored only when **every** finalized tool result in the batch sets it.
- Low-level `run_loop` / `run_loop_continue` exist as observational variants (no message_end barrier between producer phases) for parity with the TS `agentLoop()` / `agentLoopContinue()` API.