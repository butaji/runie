# PI → Runie Architecture Alignment Document

**Date:** 2026-05-27
**Status:** Draft — Architectural mapping for runie v0.2 refactor

---

## Executive Summary

This document maps pi's proven TypeScript architecture to runie's Rust/actor model. Pi handles the same domain (TUI agent IDE) with ~2,500 lines of core code vs runie's ~5,000+. The goal is not blind porting but **selective adoption** of patterns that reduce complexity while preserving Rust-specific strengths.

**Key insight:** Pi achieves simplicity through:
1. **Callback-based event emission** (no channels for uni-directional flows)
2. **Agent-owned local state** (no global state atom)
3. **Width-aware string rendering** with differential updates
4. **AbortSignal cooperative cancellation** (no task abortion)
5. **EventStream as return type** (pull vs push inversion)

---

## 1. Component Interface

### pi Pattern

```typescript
// tui.ts:39-63
export interface Component {
  render(width: number): string[];
  handleInput?(data: string): void;
  wantsKeyRelease?: boolean;
  invalidate(): void;
}
```

- **Pure function:** `render(width)` → `string[]` (lines)
- **Width is explicit:** Components must respect viewport width
- **Input optional:** Only focusable components handle input
- **Invalidation:** Cache-busting signal for re-render from scratch
- **Container composes:** `Container.render()` concatenates child lines

**TUI render loop** (tui.ts:953-1280):
- Differential rendering: compares `previousLines` vs `newLines`
- Finds `firstChanged`/`lastChanged`, only rewrites changed lines
- Kitty image lifecycle tracking
- Cursor marker extraction (`CURSOR_MARKER` APC sequence)
- Throttled via `requestRender()` → `scheduleRender()` (min 16ms interval)

### Current Runie Pattern

```rust
// crates/runie-tui/src/components/component.rs:24-35
pub trait Component {
    type ViewModel;
    fn render(&self, vm: &Self::ViewModel, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper);
}
```

- **Buffer-based:** Writes directly to ratatui `Buffer`
- **Area-aware:** Receives `Rect` (x, y, w, h)
- **ViewModel associated type:** Each component declares its VM
- **No invalidation:** Ratatui handles buffer diffing internally

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `render(width): string[]` | `render(vm, area, buf, theme)` | **Keep runie.** Buffer-based is more powerful for complex layouts. But adopt pi's **width-as-constraint** discipline — components must not overflow. |
| `handleInput?(data: string)` | Event routing in `events.rs` | **Adopt pi's focused input model.** Currently runie routes all keys globally. Pi's focused-component input with `wantsKeyRelease` opt-in is cleaner. |
| `invalidate()` | N/A (ratatui handles) | **Keep runie.** No need — ratatui's `Terminal::draw()` does full buffer diff. |
| `Container` composes children | `Tui::render_*` methods | **Adopt pi's Container.** Runie hard-codes layout in `Tui::render_normal_mode`. A `Container` that holds `Box<dyn Component>` would enable dynamic plugin UI. |
| Differential line rendering | Ratatui buffer diff | **Keep runie.** Ratatui's backend diffing is sufficient. |
| `requestRender()` throttling | Render on every `Msg` | **Adopt pi's throttling.** Currently runie renders after every message including `Tick` (80ms) and `CursorBlink` (500ms). Pi's 16ms min interval prevents excessive redraws. |

### Specific File Changes

**`crates/runie-tui/src/components/component.rs`**
- Add `handle_input(&mut self, key: KeyEvent) -> Option<Msg>` method to `Component` trait
- Add `wants_focus(&self) -> bool` defaulting to false
- Create `Container` struct that holds `Vec<Box<dyn Component>>` and implements `Component` by concatenating render output

**`crates/runie-tui/src/tui.rs`**
- Replace hard-coded `render_normal_mode` with a root `Container`
- Add `request_render()` method with 16ms throttle instead of rendering on every message
- Add `focused_component: Option<usize>` index into container children
- Route input to focused component first, then fall through to global handlers

**`crates/runie-tui/src/tui/events.rs`**
- Replace global mode-based routing with focused-component routing
- Only if no component consumes the key, apply global shortcuts

---

## 2. Agent Loop

### pi Pattern

```typescript
// agent-loop.ts:31-54
export function agentLoop(
  prompts: AgentMessage[],
  context: AgentContext,
  config: AgentLoopConfig,
  signal?: AbortSignal,
  streamFn?: StreamFn,
): EventStream<AgentEvent, AgentMessage[]> {
  const stream = createAgentStream();
  void runAgentLoop(prompts, context, config, async (event) => {
    stream.push(event);
  }, signal, streamFn).then((messages) => {
    stream.end(messages);
  });
  return stream;
}
```

- **Returns EventStream:** Caller pulls events via `for await`
- **Push internally:** Loop pushes events to stream via `emit` callback
- **AsyncIterable interface:** Consumer uses `for await (const event of stream)`
- **Final result:** `stream.result()` resolves to `AgentMessage[]`
- **Two entry points:** `agentLoop()` (new prompt) and `agentLoopContinue()` (resume)

**Inner loop structure** (agent-loop.ts:155-269):
```
while (true) {                    // Outer: follow-up messages
  while (hasMoreToolCalls || pendingMessages.length > 0) {
    // Process steering messages
    // Stream assistant response
    // Execute tool calls (sequential or parallel)
    // Emit turn_end
    // Check shouldStopAfterTurn
  }
  // Check follow-up messages
}
```

### Current Runie Pattern

```rust
// crates/runie-agent/src/loop_engine.rs:57-66
pub async fn run_agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: &dyn Provider,
    tools: &[AgentTool],
    mut event_rx: mpsc::Receiver<AgentEvent>,    // ← Bi-directional channel
    forwarder: Arc<dyn EventForwarder>,           // ← Event sink trait
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<(), AgentLoopError>
```

- **Async function with side effects:** Takes `EventForwarder`, calls `forward()` for each event
- **Bi-directional channel:** `event_rx` receives `PermissionDecision` events from UI
- **No return stream:** Events are pushed out via callback
- **Single entry point:** No distinction between new prompt and continuation
- **Tight coupling:** Loop owns permission flow, blocking on `event_rx.recv()`

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `EventStream<AgentEvent, AgentMessage[]>` | `impl Stream<Item = AgentEvent>` + `Future<Output = Vec<AgentMessage>>` | **Adopt pi's pattern.** Return a stream+result pair instead of taking a callback. Decouples loop from UI. |
| `emit(event)` callback | `EventForwarder` trait + mpsc | **Replace with stream.** Callbacks create inversion of control. A returned stream lets caller decide routing. |
| `agentLoop` vs `agentLoopContinue` | Single `run_agent_loop` | **Adopt pi's dual entry.** Add `run_agent_loop_continue()` for resuming from existing transcript. |
| `signal?: AbortSignal` | `JoinHandle::abort()` | **Adopt AbortSignal.** See Section 5. |
| `getSteeringMessages()` | Inline permission check | **Adopt pi's steering queue.** Decouple permission system from loop via pre-turn message injection. |
| `shouldStopAfterTurn()` | `max_turns` counter | **Adopt pi's hook.** `shouldStopAfterTurn` is more flexible than a hard turn limit. |

### Specific File Changes

**`crates/runie-agent/src/loop_engine.rs`** — Complete rewrite recommended

```rust
// New API inspired by pi
pub struct AgentEventStream {
    rx: mpsc::Receiver<AgentEvent>,
    result: Shared<BoxFuture<'static, Vec<AgentMessage>>>,
}

impl Stream for AgentEventStream {
    type Item = AgentEvent;
    // ...
}

impl AgentEventStream {
    pub async fn result(self) -> Vec<AgentMessage> {
        // ...
    }
}

pub fn agent_loop(
    prompts: Vec<AgentMessage>,
    context: AgentContext,
    config: AgentLoopConfig,
    cancel: CancellationToken,  // Rust equivalent of AbortSignal
    stream_fn: Option<StreamFn>,
) -> AgentEventStream {
    let (tx, rx) = mpsc::channel(128);
    let (result_tx, result_rx) = oneshot::channel();
    
    tokio::spawn(async move {
        let messages = run_loop(prompts, context, config, |event| {
            let _ = tx.try_send(event);
        }, cancel, stream_fn).await;
        let _ = result_tx.send(messages);
    });
    
    AgentEventStream { rx, result: result_rx.shared() }
}

// Internal loop matching pi's structure
async fn run_loop(
    context: AgentContext,
    config: AgentLoopConfig,
    emit: impl Fn(AgentEvent),
    cancel: CancellationToken,
    stream_fn: Option<StreamFn>,
) -> Vec<AgentMessage> {
    // Outer loop for follow-ups
    // Inner loop for turns + tool calls
    // Steering message injection at turn boundaries
}
```

**`crates/runie-cli/src/tui_run.rs`**
- Remove `EventBridge` struct and `EventForwarder` trait
- Replace `agent_task: Option<JoinHandle<()>>` with `agent_stream: Option<AgentEventStream>`
- Consume stream via `while let Some(event) = agent_stream.next().await`
- Send permission decisions through a separate `mpsc::Sender<PermissionDecision>` injected into config

---

## 3. Event Emission

### pi Pattern

```typescript
// agent-loop.ts:25
export type AgentEventSink = (event: AgentEvent) => Promise<void> | void;
```

- **Direct callback:** `emit(event)` is a function call, not a channel send
- **Awaited:** Loop `await emit(event)` — backpressure is natural
- **Synchronous or async:** Sink can be sync or return a Promise
- **No serialization:** Events are passed by reference (same process)

**Usage in loop** (agent-loop.ts:109-114):
```typescript
await emit({ type: "agent_start" });
await emit({ type: "turn_start" });
for (const prompt of prompts) {
  await emit({ type: "message_start", message: prompt });
  await emit({ type: "message_end", message: prompt });
}
```

### Current Runie Pattern

```rust
// crates/runie-agent/src/loop_engine.rs:52-55
pub trait EventForwarder: Send + Sync {
    fn forward(&self, event: AgentEvent);
}
```

- **Trait object:** `Arc<dyn EventForwarder>` shared across async boundary
- **Fire-and-forget:** `forward()` returns `()`, no backpressure
- **Bridged via channel:** `EventBridge` converts to `Msg` and `try_send`s to mpsc
- **Two channels:** One for events out (`EventForwarder`), one for decisions in (`event_rx`)

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `emit(event)` callback | `Fn(AgentEvent)` closure | **Adopt closure-based emission.** Simpler than trait object + channel bridge. |
| `await emit(event)` | `emit(event).await` if async | **Adopt awaited emission.** Current `forward()` is fire-and-forget. Awaiting ensures backpressure and ordered delivery. |
| No channel for events | `mpsc` channel | **Remove channel for uni-directional events.** Channels add overhead and require `Clone` on events. Direct callback is cheaper and clearer. |
| Channel for bi-directional | `mpsc` for permissions | **Keep channel for permissions only.** Permission decisions are true async requests from UI to agent. |

### Specific File Changes

**`crates/runie-agent/src/loop_engine.rs`**
- Remove `EventForwarder` trait
- Change `run_agent_loop` signature:
  ```rust
  pub async fn run_agent_loop(
      prompts: Vec<AgentMessage>,
      context: AgentContext,
      config: AgentLoopConfig,
      emit: impl Fn(AgentEvent) + Send + Sync + 'static,
      permission_rx: mpsc::Receiver<PermissionDecision>,
      cancel: CancellationToken,
  ) -> Vec<AgentMessage>
  ```
- All `forwarder.forward(event)` becomes `emit(event)`
- Permission decisions come through dedicated `permission_rx`, not mixed with `AgentEvent`

**`crates/runie-agent/src/events.rs`**
- Remove `PermissionDecision` from `AgentEvent` enum
- Create separate `PermissionRequest` / `PermissionResponse` types

**`crates/runie-cli/src/tui_run.rs`**
- Remove `EventBridge` and `EventForwarder` impl
- Pass closure directly: `|event| { let _ = msg_tx.try_send(Msg::AgentEvent(event)); }`

---

## 4. State Management

### pi Pattern

```typescript
// agent.ts:59-93
type MutableAgentState = Omit<AgentState, "isStreaming" | ...> & {
  isStreaming: boolean;
  streamingMessage?: AgentMessage;
  pendingToolCalls: Set<string>;
  errorMessage?: string;
};

function createMutableAgentState(initialState?: Partial<...>): MutableAgentState {
  let tools = initialState?.tools?.slice() ?? [];
  let messages = initialState?.messages?.slice() ?? [];
  return {
    systemPrompt: initialState?.systemPrompt ?? "",
    model: initialState?.model ?? DEFAULT_MODEL,
    get tools() { return tools; },
    set tools(nextTools: AgentTool<any>[]) { tools = nextTools.slice(); },
    get messages() { return messages; },
    set messages(nextMessages: AgentMessage[]) { messages = nextMessages.slice(); },
    // ...
  };
}
```

- **Agent owns state:** `Agent` class is the source of truth
- **Getters/setters with copy-on-write:** Assigning `messages` or `tools` copies the array
- **Readonly public interface:** `AgentState` interface exposes readonly fields
- **Mutable internal:** `_state` is mutable for loop mutations
- **No global store:** Each `Agent` instance is independent
- **Event-driven updates:** State changes via `processEvents()` reacting to loop events

**Agent class API** (agent.ts:166-557):
```typescript
class Agent {
  private _state: MutableAgentState;
  private listeners: Set<(...)>;
  private steeringQueue: PendingMessageQueue;
  private followUpQueue: PendingMessageQueue;
  private activeRun?: ActiveRun;

  subscribe(listener): () => void;
  get state(): AgentState;
  steer(message): void;
  followUp(message): void;
  prompt(message): Promise<void>;
  continue(): Promise<void>;
  abort(): void;
  waitForIdle(): Promise<void>;
  reset(): void;
}
```

### Current Runie Pattern

```rust
// crates/runie-agent/src/state.rs:1-22
pub struct AgentState {
    pub session: Session,
    pub working_memory: WorkingMemory,
    pub turn_count: usize,
}
```

- **Thin state struct:** Just `Session` + `WorkingMemory` + counter
- **Loop owns messages:** `loop_engine.rs` mutates `messages: Vec<AgentMessage>` directly
- **Global AppState:** `runie-tui/src/tui/state.rs` has monolithic `AppState` with 20+ fields
- **TEA pattern:** `update(state, msg) -> Vec<Cmd>` — Redux-like reducer

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `Agent` class owns state | `Agent` struct owns state | **Adopt pi's Agent struct.** Currently runie has `AgentState` (data) but no `Agent` (behavior). Create an `Agent` struct that wraps state + loop control. |
| Getters/setters with COW | `messages()` / `set_messages()` | **Adopt in Agent API.** Rust's ownership makes this natural. Return `&[AgentMessage]` (immutable borrow), `set_messages` clones. |
| No global store | `AppState` with 20+ fields | **Split AppState.** Extract agent-related state into `Agent`. Keep UI-only state in `AppState`. |
| `subscribe(listener)` | `tokio::sync::broadcast` or callback | **Adopt callback subscription.** `Agent::subscribe(fn(AgentEvent))` matches pi's pattern. |
| `steer()` / `followUp()` queues | N/A | **Adopt steering/follow-up queues.** Currently runie has no mid-run user message injection. |
| `processEvents()` reduces state | Inline in `update::agent` | **Move to Agent.** State reduction from events should happen in `Agent`, not TUI reducer. |

### Specific File Changes

**`crates/runie-agent/src/lib.rs`** — New `Agent` struct

```rust
pub struct Agent {
    state: MutableAgentState,
    listeners: Vec<Box<dyn Fn(&AgentEvent) + Send + Sync>>,
    steering_queue: PendingMessageQueue,
    follow_up_queue: PendingMessageQueue,
    active_run: Option<ActiveRun>,
}

impl Agent {
    pub fn new(initial_state: Option<AgentState>) -> Self;
    pub fn subscribe<F>(&mut self, listener: F) -> SubscriptionHandle where F: Fn(&AgentEvent) + Send + Sync + 'static;
    pub fn steer(&mut self, message: AgentMessage);
    pub fn follow_up(&mut self, message: AgentMessage);
    pub async fn prompt(&mut self, message: impl Into<AgentMessage>) -> Result<(), AgentError>;
    pub async fn continue_(&mut self) -> Result<(), AgentError>;
    pub fn abort(&mut self);
    pub async fn wait_for_idle(&self);
    pub fn reset(&mut self);
    pub fn state(&self) -> &AgentState;
}
```

**`crates/runie-agent/src/state.rs`** — Expand to match pi's `AgentState`

```rust
pub struct AgentState {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: ThinkingLevel,
    tools: Vec<AgentTool>,
    messages: Vec<AgentMessage>,
    pub is_streaming: bool,
    pub streaming_message: Option<AgentMessage>,
    pub pending_tool_calls: HashSet<String>,
    pub error_message: Option<String>,
}

impl AgentState {
    pub fn tools(&self) -> &[AgentTool] { &self.tools }
    pub fn set_tools(&mut self, tools: Vec<AgentTool>) { self.tools = tools; }
    pub fn messages(&self) -> &[AgentMessage] { &self.messages }
    pub fn set_messages(&mut self, messages: Vec<AgentMessage>) { self.messages = messages; }
}
```

**`crates/runie-tui/src/tui/state.rs`** — Slim down `AppState`
- Remove `messages: Vec<MessageItem>` — delegate to `Agent::state().messages()`
- Remove `agent_running` — use `Agent::state().is_streaming`
- Remove `token_usage` — derive from agent events
- Keep UI-only state: `textarea`, `mode`, `scroll`, `diff_viewer`, `onboarding`, etc.

**`crates/runie-tui/src/tui/update/agent.rs`** — Simplify
- Remove state reduction logic (moved to `Agent::process_event()`)
- Just forward events: `agent.subscribe(|event| msg_tx.try_send(Msg::AgentEvent(event.clone())))`

---

## 5. Cancellation

### pi Pattern

```typescript
// agent-loop.ts:35
export function agentLoop(
  ...,
  signal?: AbortSignal,
): EventStream<AgentEvent, AgentMessage[]> {

// agent-loop.ts:440-442
if (signal?.aborted) {
  break;
}

// agent.ts:300-302
abort(): void {
  this.activeRun?.abortController.abort();
}
```

- **AbortSignal passed through entire stack:** From `Agent.abort()` → `agentLoop()` → `streamAssistantResponse()` → `streamFunction()` → tool execution
- **Cooperative:** Code checks `signal.aborted` at natural boundaries (between tool calls, after stream events)
- **No exceptions:** Aborted operations return normally with `stopReason: "aborted"`
- **Per-run signal:** Each `activeRun` gets its own `AbortController`

### Current Runie Pattern

```rust
// crates/runie-cli/src/tui_run.rs:377-382
Cmd::Interrupt => {
    if let Some(handle) = agent_task.take() {
        handle.abort();
        let _ = handle.await;
    }
    vec![]
}
```

- **Task abortion:** `JoinHandle::abort()` forcefully cancels the tokio task
- **No cleanup signal:** Loop doesn't know it's being aborted
- **Panic risk:** `abort()` can leave resources in inconsistent state
- **No graceful degradation:** Partial assistant messages may be lost

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `AbortSignal` | `tokio_util::sync::CancellationToken` | **Adopt CancellationToken.** This is Rust's closest equivalent to AbortSignal — cooperative, clonable, composable. |
| `signal.aborted` | `cancel.is_cancelled()` | **Adopt cooperative checks.** Add cancellation checks at turn boundaries and between tool calls. |
| `AbortController` | `CancellationToken::new()` + `child_token()` | **Adopt per-run token.** Each `Agent::prompt()` creates a child token. `Agent::abort()` calls `cancel()`. |
| No task abortion | `JoinHandle::abort()` | **Remove task abortion.** Let the loop exit gracefully via cancellation checks. |

### Specific File Changes

**`crates/runie-agent/src/lib.rs`** — Add to `Agent`

```rust
use tokio_util::sync::CancellationToken;

pub struct Agent {
    // ...
    cancel_token: Option<CancellationToken>,
}

impl Agent {
    pub fn abort(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
    }
    
    pub fn signal(&self) -> Option<CancellationToken> {
        self.cancel_token.clone()
    }
}
```

**`crates/runie-agent/src/loop_engine.rs`**
- Replace `JoinHandle` abort with `CancellationToken` checks:
  ```rust
  if cancel.is_cancelled() {
      return build_aborted_message(messages);
  }
  ```
- Check at:
  - Start of each turn
  - Between tool call preparations
  - Between tool call executions
  - After each stream event (with `tokio::select!` on stream next + cancellation)

**`crates/runie-cli/src/tui_run.rs`**
- Remove `Cmd::Interrupt` task abortion
- `Agent::abort()` calls `token.cancel()`
- Loop exits gracefully, emits `AgentEnd` with partial state

---

## 6. TUI Loop

### pi Pattern

```typescript
// tui.ts:441-450
start(): void {
  this.stopped = false;
  this.terminal.start(
    (data) => this.handleInput(data),
    () => this.requestRender(),
  );
  this.terminal.hideCursor();
  this.queryCellSize();
  this.requestRender();
}

// tui.ts:495-542
requestRender(force = false): void {
  if (force) { /* immediate render */ return; }
  if (this.renderRequested) return;
  this.renderRequested = true;
  process.nextTick(() => this.scheduleRender());
}

private scheduleRender(): void {
  const elapsed = performance.now() - this.lastRenderAt;
  const delay = Math.max(0, TUI.MIN_RENDER_INTERVAL_MS - elapsed);
  this.renderTimer = setTimeout(() => { this.doRender(); }, delay);
}
```

- **Terminal owns input callback:** `terminal.start(onInput, onResize)`
- **Request-render pattern:** Components don't render directly; they call `requestRender()`
- **Throttled:** Minimum 16ms between renders (matches 60fps)
- **Forced render:** `requestRender(true)` clears all caches and redraws immediately
- **Input → render chain:** `handleInput()` calls `focusedComponent.handleInput()`, then `requestRender()`

### Current Runie Pattern

```rust
// crates/runie-cli/src/tui_run.rs:409-472
while tui.state.running {
    tokio::select! {
        biased;
        Some(msg) = msg_rx.recv() => {
            let cmds = tui.update(msg);
            // process cmds...
            tui.render()?;  // ← Render after EVERY message
        }
        _ = cursor_interval.tick() => {
            let cmds = tui.update(Msg::CursorBlink);
            tui.render()?;
        }
        _ = tick_interval.tick() => {
            let cmds = tui.update(Msg::Tick);
            tui.render()?;
        }
    }
}
```

- **Message-driven render:** Every message triggers render
- **No throttling:** `Tick` every 80ms + `CursorBlink` every 500ms + key events = many renders
- **Dirty flag exists but unused for throttling:** `Tui::dirty` is set in `update()` but render happens unconditionally
- **Alternative screen:** Uses ratatui's alternate screen

### Mapping

| pi Concept | Rust Equivalent | Decision |
|-----------|----------------|----------|
| `terminal.start(onInput, onResize)` | `tokio::task` reading crossterm events | **Keep runie's channel approach.** It's more Rust-idiomatic than callbacks. |
| `requestRender()` | `Tui::request_render()` | **Adopt request-render pattern.** Currently renders on every message. Batch rapid messages. |
| 16ms throttle | `tokio::time::interval` or `sleep` | **Adopt 16ms throttle.** Reduces CPU usage and flicker. |
| `handleInput()` → focused component | `events.rs` mode routing | **Adopt focused-component input.** See Section 1. |
| `process.nextTick()` | `tokio::task::yield_now()` | **Use yield in async context.** Equivalent deferral mechanism. |

### Specific File Changes

**`crates/runie-tui/src/tui.rs`**
```rust
impl Tui {
    const MIN_RENDER_INTERVAL_MS: u64 = 16;
    
    pub fn request_render(&mut self, force: bool) {
        if force {
            self.dirty = true;
            self.render_immediate();
            return;
        }
        if self.dirty { return; }
        self.dirty = true;
        // Defer to next async iteration — runtime will batch
    }
    
    pub async fn render_if_dirty(&mut self) -> io::Result<()> {
        if !self.dirty { return Ok(()); }
        self.dirty = false;
        self.render().await
    }
}
```

**`crates/runie-cli/src/tui_run.rs`**
```rust
while tui.state.running {
    tokio::select! {
        biased;
        Some(msg) = msg_rx.recv() => {
            let cmds = tui.update(msg);
            // process cmds...
            tui.request_render(false);
        }
        _ = cursor_interval.tick() => {
            let cmds = tui.update(Msg::CursorBlink);
            tui.request_render(false);
        }
        _ = tick_interval.tick() => {
            let cmds = tui.update(Msg::Tick);
            tui.request_render(false);
        }
        // Render throttle
        _ = tokio::time::sleep(Duration::from_millis(16)) => {
            tui.render_if_dirty().await?;
        }
    }
}
```

---

## 7. Additional Patterns to Adopt

### 7.1 Tool Execution Modes

**pi:** Supports both "sequential" and "parallel" tool execution with per-tool override via `executionMode`.

**runie:** Has `ToolExecutionMode` enum but loop engine always executes sequentially.

**Action:** Implement parallel tool execution in loop engine. Use `tokio::join!` or `FuturesUnordered`.

### 7.2 Before/After Tool Call Hooks

**pi:** `beforeToolCall` and `afterToolCall` hooks in `AgentLoopConfig` with full context.

**runie:** Has `Hook` trait with `before_tool_call`/`after_tool_call` but loop engine integrates manually.

**Action:** Align hook signatures with pi's context-rich approach. Pass `AgentContext` to hooks.

### 7.3 Steering and Follow-Up Queues

**pi:** `steer()` injects messages mid-run. `followUp()` queues messages for after natural stop.

**runie:** No equivalent. Permission system blocks loop; no mid-run user input.

**Action:** Add `Agent::steer()` and `Agent::follow_up()`. Loop polls steering queue at turn boundaries.

### 7.4 Context Transform

**pi:** `transformContext` hook prunes/mutates messages before LLM call.

**runie:** No equivalent. Messages grow unbounded.

**Action:** Add `transform_context` config hook. Integrate with existing `Compactor`.

### 7.5 EventStream as AsyncIterable

**pi:** `EventStream` implements `AsyncIterable`, enabling `for await` consumption.

**runie:** Events pushed via callback; no stream abstraction.

**Action:** Implement `Stream` for `AgentEventStream`. Enables `.for_each()`, `.filter()`, etc.

---

## 8. What Runie Should Keep

| Feature | Why Keep It |
|--------|-------------|
| **Ratatui buffer rendering** | More powerful than string-based. Handles complex layouts, styling, Unicode width correctly. |
| **TEA pattern (Msg/Cmd/update)** | Excellent for UI state management. Testable, predictable, time-travel debuggable. |
| **Rust type system** | Stronger guarantees than TypeScript. `AgentEvent` as enum vs tagged union is safer. |
| **Tokio async runtime** | True parallelism for tool execution. Pi's JS is single-threaded. |
| **Workspace/ToolRegistry** | Rust's module system enables clean tool registration. |
| **Session tree / WorkingMemory** | More sophisticated than pi's flat message array. |
| **ViewModel pattern** | Separation of render data from state enables optimization and testing. |
| **Theme system** | DESIGN.md's token-based theming is more sophisticated than pi's ad-hoc colors. |

---

## 9. Migration Priority

### Phase 1: Agent Core (Highest Impact)
1. Create `Agent` struct with owned state, getters/setters
2. Implement `AgentEventStream` returning `impl Stream<Item = AgentEvent>`
3. Replace `EventForwarder` with emit callback
4. Add `CancellationToken` cooperative cancellation
5. Add `steer()` / `follow_up()` queues

**Files:** `crates/runie-agent/src/lib.rs`, `state.rs`, `loop_engine.rs`, `events.rs`

### Phase 2: TUI Integration
1. Simplify `AppState` — remove agent-owned fields
2. Update `tui_run.rs` to consume `AgentEventStream`
3. Remove `EventBridge`, pass closure directly
4. Implement `request_render()` throttling

**Files:** `crates/runie-tui/src/tui/state.rs`, `crates/runie-cli/src/tui_run.rs`

### Phase 3: Component Model
1. Add `handle_input` to `Component` trait
2. Implement focused-component routing
3. Create `Container` for dynamic layouts
4. Add overlay stack (modeled after pi's `overlayStack`)

**Files:** `crates/runie-tui/src/components/component.rs`, `crates/runie-tui/src/tui/events.rs`, `crates/runie-tui/src/tui.rs`

### Phase 4: Polish
1. Parallel tool execution
2. `transformContext` hook
3. `AgentLoopConfig` alignment with pi's options
4. Performance: measure render times, optimize `ViewModels::from_render_state`

---

## 10. Cross-Reference: pi File → runie File Mapping

| pi File | Pattern | runie Target File(s) |
|--------|---------|---------------------|
| `packages/tui/src/tui.ts` | Component interface, TUI loop, differential render | `crates/runie-tui/src/components/component.rs`, `crates/runie-tui/src/tui.rs` |
| `packages/agent/src/agent-loop.ts` | Agent loop, EventStream, tool execution | `crates/runie-agent/src/loop_engine.rs` |
| `packages/agent/src/agent.ts` | Agent class, state management, queues | `crates/runie-agent/src/lib.rs`, `state.rs` |
| `packages/agent/src/types.ts` | Agent types, config, hooks | `crates/runie-agent/src/events.rs`, `config.rs`, `hook.rs` |
| `packages/ai/src/utils/event-stream.ts` | EventStream async iterable | New: `crates/runie-agent/src/event_stream.rs` |

---

## Appendix: Side-by-Side Code Comparison

### Agent Loop Return Type

**pi:**
```typescript
function agentLoop(...): EventStream<AgentEvent, AgentMessage[]> {
  const stream = createAgentStream();
  void runAgentLoop(..., async (event) => { stream.push(event); }, ...)
    .then((messages) => { stream.end(messages); });
  return stream;
}

// Usage:
const stream = agentLoop(prompts, context, config);
for await (const event of stream) {
  render(event);
}
const messages = await stream.result();
```

**runie (proposed):**
```rust
pub fn agent_loop(
    prompts: Vec<AgentMessage>,
    context: AgentContext,
    config: AgentLoopConfig,
    cancel: CancellationToken,
) -> AgentEventStream {
    let (tx, rx) = mpsc::channel(128);
    let (result_tx, result_rx) = oneshot::channel();
    
    tokio::spawn(async move {
        let messages = run_loop(prompts, context, config, |event| {
            let _ = tx.try_send(event);
        }, cancel).await;
        let _ = result_tx.send(messages);
    });
    
    AgentEventStream { rx, result: result_rx.shared() }
}

// Usage:
let mut stream = agent.prompt(message).await?;
while let Some(event) = stream.next().await {
    render(&event);
}
let messages = stream.result().await;
```

### Event Emission

**pi:**
```typescript
await emit({ type: "message_start", message: prompt });
```

**runie (current):**
```rust
forwarder.forward(AgentEvent::MessageStart { message: prompt.clone() });
```

**runie (proposed):**
```rust
emit(AgentEvent::MessageStart { message: prompt.clone() }).await;
```

### Cancellation

**pi:**
```typescript
const controller = new AbortController();
agentLoop(..., controller.signal, ...);
controller.abort();  // Cooperative
```

**runie (current):**
```rust
let handle = tokio::spawn(async { run_agent_loop(...).await });
handle.abort();  // Forceful, risky
```

**runie (proposed):**
```rust
let cancel = CancellationToken::new();
agent_loop(..., cancel.clone(), ...);
cancel.cancel();  // Cooperative, safe
```

---

*End of document.*
