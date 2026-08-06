# p06 — Core loop: prompt() busy guard + continue() validation + run_loop_continue semantics

**Parity target:** pi `Agent` facade + `runAgentLoopContinue`.

## Pi reference

- `Agent.prompt(...)` — `~/Code/agents/pi/packages/agent/src/agent.ts:339`: throws `"Agent is already processing a prompt. Use steer() or followUp() to queue messages, or wait for completion."` if `activeRun` is set (line 340). Normalizes input via `normalizePromptInput` (line 379): array→as-is; single message→`[message]`; string→user text message with `timestamp: Date.now()`.
- `Agent.continue()` — `agent.ts:350`: throws if busy; if last message is assistant, drains steering then follow-up (runs those), and only errors when both queues are empty; otherwise `runContinuation()`.
- `runAgentLoopContinue` — `agent-loop.ts:120`: throws `"Cannot continue: no messages in context"` if `context.messages.length === 0` (line 127); throws `"Cannot continue from message role: assistant"` if last message `role === "assistant"` (line 131). `newMessages = []` (does **not** include pre-existing context), emits `agent_start`+`turn_start`, then continues.
- `PendingMessageQueue.drain` — `agent.ts:139`: `"all"` drains everything; `"one-at-a-time"` returns only the oldest.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/loop/driver.rs`
- `run_loop_continue` (driver.rs:266) calls `run_loop(vec![], ...)` with **no validation** and **no busy guard**.
- `LoopActor::prompt` (loop/actor.rs) — no busy guard; concurrent prompts are allowed.

## Adapt to runie

1. Channel the busy guard through `LoopActor`: track an active run (e.g. a watch/state flag on the actor). `prompt()` returns an `Err` (`LoopError::Busy`) when a run is active; `follow_up`/`steer` remain allowed (they are queue pushes).
2. Add validation to `run_loop_continue`:
   - `context.messages.is_empty()` → error `"Cannot continue: no messages in context"`.
- last message is `Assistant` with empty steering/follow-up queues → error `"Cannot continue from message role: assistant"`; queued steering/follow-up messages are drained according to queue mode and run.
3. Ensure `run_loop_continue` returns only **new** messages (starts empty, not with prompts).
4. Add `LoopActor::steer` / `follow_up` / `clear_queues` / `has_queued` mirrors of `Agent.steer/followUp/clearAllQueues/hasQueuedMessages` (agent.ts:276-302).

## State machine / variants

`LoopActor` run lifecycle:
```
idle --prompt()--> running --agent_end--> idle
running --prompt()--> Err(Busy)     (rejected)
running --steer()/follow_up()--> accepted (queued)
idle --continue()--> validate(context) --> running | Err(no messages | last is assistant)
```
`run_loop_continue` result variants: `Ok(new_messages)` (empty when nothing produced) | `Err(empty_context)` | `Err(last_is_assistant)`.

## Acceptance

- Tests: concurrent `prompt()` returns Busy; `run_loop_continue` on empty context errors; `run_loop_continue` when last message is assistant errors; `run_loop_continue` on a user-terminated context produces only new messages.
- Regression: assistant-terminated `continue_run` consumes queued steering messages before returning the new user + assistant messages.
- Continuation parity: the assistant-ending steering path skips the normal
  initial steering poll, matching pi's `skipInitialSteeringPoll` option and
  preventing an extra queue drain before the provider request.
- `cargo test -p runie-core` green.
