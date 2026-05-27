# Agent Architecture Analysis: Codex vs OpenCode vs Pi → Runie Recommendations

**Date:** 2026-05-27
**Status:** Strategic recommendation for runie v0.2 architecture
**Sources:** Codex (Rust, GitHub Copilot CLI), OpenCode (TypeScript, opencode.ai), Pi (TypeScript, earendil-works/pi)

---

## 1. Architecture Patterns Comparison Table

| Dimension | **Codex (Rust)** | **OpenCode (TypeScript)** | **Pi (TypeScript)** | **Current Runie** |
|-----------|------------------|---------------------------|---------------------|-------------------|
| **State Management** | ThreadManager owns sessions; client-server split | InstanceState per project; Effect framework for functional errors | Agent-owned local state; getters/setters with copy-on-write; no global store | Monolithic AppState (20+ fields); TEA reducer pattern; AgentState thin struct |
| **Communication** | UDS/WebSocket between TUI ↔ app-server | IPC between SolidJS desktop ↔ core; raw ANSI terminal UI | Direct callback `emit(event)`; EventStream return type; no channels for uni-directional | Bi-directional mpsc channels; EventForwarder trait object; mixed events + permissions |
| **Error Handling** | Rust Result types; structured errors | Effect framework (Either/TaskEither); explicit error channels | TypeScript exceptions + signal-based cancellation; cooperative abort | RunieError enum; panic catch in tool execution; CancellationToken |
| **Tool Execution** | Hook system (session_start, pre_tool_use, post_tool_use); MCP servers | Synthetic tool for structured JSON; permission rulesets per agent; snapshot/patch tracking | Sequential or parallel modes; before/after hooks with full context; executionMode override | Sequential only; Hook trait (before/after); ToolRegistry HashMap; duplicate detection |
| **Persistence** | SQLite via sqlx; session tree | SQLite via Drizzle ORM; per-project caching | YAML state files; session sharing via HuggingFace; proper-lockfile | In-memory only; Session struct with MessageNode tree; SimpleCompactor stub |
| **Provider Abstraction** | ModelProvider trait + BearerAuth | 13 providers with unified interface | Model registry with auto-generated defaults; OAuth device code flow | RigProvider enum (22 providers); macro dispatch; hardcoded metadata heuristics |
| **TUI Rendering** | ratatui + crossterm; buffer-based | SolidJS desktop + raw ANSI terminal; doomscroll detection | `render(width) → string[]`; differential line rendering; 16ms throttle | ratatui buffer-based; ViewModel pattern; 16ms throttle (tui_run.rs) |
| **Context Management** | Skills system for prompt injection | Context compaction; token tracking | Context compaction via summarization; transformContext hook | WorkingMemory struct; SimpleCompactor stub; enable_compaction flag |
| **Sandboxing** | landlock + Windows isolation | Permission rulesets; PTY/shell integration | Not a focus (relies on OS) | SafetyHook (bash string matching); no filesystem sandbox |
| **Session Model** | Client-server; session_start hooks | Session processor; doomscroll detection | Agent class owns state; steer/followUp queues; resume capability | Session tree (parent_id links); add_message/get_thread; no resume |

---

## 2. Top 10 Patterns to Adopt for Runie

Ranked by **impact vs effort** (1 = highest impact/effort ratio).

### 1. **Cooperative Cancellation (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie currently uses `JoinHandle::abort()` which is forceful and risky. Pi's `AbortSignal` pattern cooperatively exits at natural boundaries (turn end, between tool calls), preserving partial state.
- **Implementation notes:** Replace task abortion with `tokio_util::sync::CancellationToken` checks in loop_engine.rs. Check at: start of each turn, between tool calls, after stream events. Already partially implemented — complete the migration.
- **Effort:** Low | **Impact:** High

### 2. **Steering/Follow-Up Message Queues (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie blocks on permission modals with no mid-run user input. Pi's `steer()` injects messages mid-run; `followUp()` queues for after natural stop. Enables true interactivity.
- **Implementation notes:** Add `steering_queue: VecDeque<AgentMessage>` and `follow_up_queue: VecDeque<AgentMessage>` to Agent struct. Poll steering queue at turn boundaries. `followUp` feeds into next prompt.
- **Effort:** Medium | **Impact:** High

### 3. **Agent Struct with Owned State (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie has `AgentState` (data) but no `Agent` (behavior). Pi's `Agent` class is the source of truth with `prompt()`, `continue()`, `abort()`, `waitForIdle()`. Decouples agent logic from TUI.
- **Implementation notes:** Create `Agent` struct wrapping `AgentState`, queues, active run handle. Expose `subscribe(fn(AgentEvent))` for event-driven UI updates. Move state reduction from TUI update reducer into `Agent::process_event()`.
- **Effort:** Medium | **Impact:** High

### 4. **Parallel Tool Execution (Pi + OpenCode)**
- **Which agent:** Pi (sequential/parallel modes), OpenCode (synthetic tool parallelism)
- **Why adopt:** Runie has `ToolExecutionMode::Parallel` enum variant but loop engine always executes sequentially. Independent tool calls (e.g., read_file × 3) should run concurrently.
- **Implementation notes:** Use `tokio::join!` or `FuturesUnordered` for tool calls marked parallel. Maintain order in message history. Respect `executionMode` override per tool config.
- **Effort:** Medium | **Impact:** Medium-High

### 5. **Hook System with Rich Context (Codex + Pi)**
- **Which agent:** Codex (session_start, pre_tool_use, post_tool_use), Pi (beforeToolCall, afterToolCall with AgentContext)
- **Why adopt:** Runie's hooks receive minimal context. Codex passes full session state; Pi passes `AgentContext`. Enables hooks that make intelligent decisions (e.g., "block bash rm when git status shows uncommitted changes").
- **Implementation notes:** Expand `Hook::before_tool_call` signature to include `&AgentState` and `&AgentContext`. Add `session_start`, `turn_start`, `turn_end` hook points. Codex-style lifecycle hooks are more comprehensive than Pi's tool-only hooks.
- **Effort:** Medium | **Impact:** Medium-High

### 6. **EventStream Return Type (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie's `agent_loop()` takes `EventForwarder` callback — inversion of control. Pi returns `EventStream<AgentEvent, AgentMessage[]>`; caller pulls events via `for await`. Decouples loop from UI routing.
- **Implementation notes:** `AgentEventStream` already exists in runie. Complete the refactor: remove `EventForwarder` trait, return `AgentEventStream` from `agent_loop()`, consume via `while let Some(event) = stream.next().await` in tui_run.rs.
- **Effort:** Low-Medium | **Impact:** Medium

### 7. **Request-Render Throttling (Pi)**
- **Which agent:** Pi
- **Why adopt:** Pi's TUI uses `requestRender()` with 16ms minimum interval. Runie already has this in tui_run.rs but should make it a first-class TUI method, not just a runtime hack.
- **Implementation notes:** Add `Tui::request_render(force: bool)` method. Batch rapid messages (Tick, CursorBlink, MessageUpdate) into single render. Force=true for mode changes and permission requests.
- **Effort:** Low | **Impact:** Medium

### 8. **Model Registry with Auto-Generated Defaults (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie hardcodes `supports_tools()`, `supports_vision()`, `max_context_tokens()` with string matching heuristics. Pi auto-generates model metadata from provider APIs.
- **Implementation notes:** Leverage rig's `ModelLister` for supported providers. Cache model metadata in `ModelRegistry`. Fetch at runtime for OpenAI/Anthropic/Gemini/DeepSeek/OpenRouter/Ollama. Fall back to heuristics only when API unavailable.
- **Effort:** Medium | **Impact:** Medium

### 9. **TransformContext Hook (Pi)**
- **Which agent:** Pi
- **Why adopt:** Runie messages grow unbounded until SimpleCompactor kicks in. Pi's `transformContext` hook prunes/mutates messages before each LLM call, enabling proactive context management.
- **Implementation notes:** Add `transform_context: Option<fn(&mut Vec<AgentMessage>, &AgentContext)>` to `AgentLoopConfig`. Call before building LLM messages. Integrate with existing `Compactor` trait — make compactor the default transform.
- **Effort:** Low | **Impact:** Medium

### 10. **SQLite Persistence + Session Resume (Codex + OpenCode)**
- **Which agent:** Codex (sqlx), OpenCode (Drizzle ORM)
- **Why adopt:** Runie is in-memory only. Sessions lost on crash. Codex persists full session tree; OpenCode has per-project InstanceState caching.
- **Implementation notes:** Add `runie-persistence` crate. Use `sqlx` with SQLite. Schema: sessions (id, created_at, metadata), messages (id, session_id, parent_id, role, content, timestamp). Implement `SessionStore` trait. Start with save-on-exit, evolve to periodic checkpointing.
- **Effort:** High | **Impact:** High

---

## 3. Specific Recommendations by Category

### Architecture

**What structural changes?**

1. **Create an `Agent` struct** that owns behavior, not just data. Pi's `Agent` class (`agent.ts:166-557`) wraps state, queues, active run, and listeners. Runie should move from `AgentState` (passive data) to `Agent` (active controller). This splits the monolithic `AppState` into UI state (`AppState`) and agent state (`Agent`).

2. **Adopt the Container pattern for TUI layout.** Pi's `Container` composes `Box<dyn Component>` dynamically. Runie hard-codes layout in `Tui::render_normal_mode` (`tui.rs:246-257`). A `Container` enables plugin UI, dynamic overlays, and easier testing.

3. **Separate permission events from agent events.** Currently `PermissionDecision` is an `AgentEvent` variant, mixed with streaming events. Pi uses a dedicated channel for bi-directional flow. Runie should split: `AgentEvent` (uni-directional: agent → UI) and `PermissionChannel` (bi-directional: UI ↔ agent).

4. **Implement MCP server integration.** Codex exposes tools via MCP. Runie has `ToolRegistry` but no MCP adapter. Add `runie-mcp` crate implementing MCP client protocol, mapping MCP tools to `runie_core::Tool` trait.

### TUI Design

**What rendering/UI patterns?**

1. **Keep ratatui buffer rendering.** Pi's `render(width) → string[]` is elegant but less powerful for complex layouts. Runie's ratatui `Buffer`-based approach (`tui.rs:163-202`) handles Unicode width, styling, and compositing correctly. Don't regress to string-based.

2. **Adopt focused-component input routing.** Pi's `handleInput?(data: string)` with `wantsKeyRelease` opt-in is cleaner than Runie's mode-based fallback routing (`events.rs:81-97`). Add `handle_input(&mut self, key: KeyEvent) → Option<Msg>` to `Component` trait. Route to focused component first, then fall through to global shortcuts.

3. **Add overlay stack.** Pi manages overlays via `overlayStack`. Runie uses flat mode enum (`TuiMode::Permission`, `TuiMode::Overlay`, etc.) which can't handle nested overlays (e.g., command palette over diff viewer). Replace with `Vec<Overlay>` stack.

4. **Implement doomscroll detection.** OpenCode's session processor detects when the user is stuck in a loop of tool calls with no progress. Add heuristic: if >5 consecutive turns with only `read_file`/`search` and no code changes, pause and prompt user.

### Model Interaction

**What provider/streaming patterns?**

1. **Complete the EventStream refactor.** Runie has `AgentEventStream` but still bridges via `EventForwarder` trait. Remove the trait, pass closure directly: `|event| { let _ = msg_tx.try_send(Msg::AgentEvent(event)); }`. This is Pi's pattern: callback for emission, not trait object + channel.

2. **Add streaming event backpressure.** Pi `await emit(event)` — backpressure is natural. Runie's `event_tx.send(...).await` already awaits, but the `EventForwarder::forward()` was fire-and-forget. Ensure all emission paths await.

3. **Implement model registry with runtime metadata.** Pi auto-generates `packages/ai/src/models.generated.ts` from provider APIs. Runie should use rig's `ModelLister` to fetch actual `supports_tools`, `supports_vision`, `max_context_tokens` at runtime for supported providers. Cache in `ModelRegistry`.

4. **Add OAuth device code flow.** Pi supports OAuth for providers that require it (e.g., some enterprise providers). Runie only supports API key auth. Add OAuth flow for providers that need it.

### Tool System

**What execution/approval patterns?**

1. **Implement parallel tool execution.** Pi supports `"executionMode": "parallel"` per tool. Runie has the enum but not the implementation. Use `tokio::join!` for independent calls. Respect ordering in message history.

2. **Add synthetic tool for structured output.** OpenCode uses a "synthetic tool" pattern: when the model needs structured JSON, it calls a special tool that validates schema against `zod`/typebox. Runie should add `StructuredOutputTool` that takes a JSON schema, validates output, and retries on schema mismatch.

3. **Expand hook lifecycle.** Codex has `session_start`, `pre_tool_use`, `post_tool_use`, `session_end`. Runie only has `before_tool_call` and `after_tool_call`. Add `on_turn_start`, `on_turn_end`, `on_session_start`, `on_session_end` to `Hook` trait. Pass `&AgentState` to all hooks for context-aware decisions.

4. **Add permission rulesets per agent.** OpenCode has granular permission rules ("allow bash in tests/", "deny write_file outside project root"). Runie has global `allowed_tools: HashSet<String>`. Replace with `PermissionRule` struct: `{ tool_pattern: String, path_pattern: Option<String>, action: Allow|Deny|Prompt }`.

### State/Persistence

**What session/state patterns?**

1. **Add SQLite persistence layer.** Codex uses `sqlx` with SQLite; OpenCode uses Drizzle ORM. Runie should add `runie-persistence` crate with `SessionStore` trait. Schema:
   ```sql
   CREATE TABLE sessions (id TEXT PRIMARY KEY, created_at INTEGER, updated_at INTEGER, metadata TEXT);
   CREATE TABLE messages (id TEXT PRIMARY KEY, session_id TEXT, parent_id TEXT, role TEXT, content TEXT, timestamp INTEGER);
   CREATE INDEX idx_session_messages ON messages(session_id, timestamp);
   ```

2. **Implement InstanceState for per-project caching.** OpenCode caches tool results, file contents, and LSP data per project. Runie's `WorkingMemory` is too small. Add `ProjectCache` with LRU eviction: file contents, search results, git status snapshots.

3. **Make compaction agent-triggered.** Pi's `transformContext` and OpenCode's context compaction are proactive. Runie's `SimpleCompactor` is reactive (at threshold). Add `compact_context` as a pseudo-tool the agent can call when it detects context pressure.

4. **Add session resume.** Pi's `agentLoopContinue()` resumes from existing transcript. Runie has no continuation. Add `Agent::continue_()` that loads messages from `SessionStore` and resumes the loop.

---

## 4. Anti-Patterns to Avoid

### From Codex — What NOT to copy

1. **115+ crate monorepo.** Codex's granularity is excessive for runie's scope. Runie's 9 crates (`runie-core`, `runie-ai`, `runie-agent`, `runie-tools`, `runie-tui`, `runie-cli`, `runie-router`, `runie-orchestrator`) is already pushing it. Don't split further without clear domain boundaries.

2. **Client-server split for local TUI.** Codex uses UDS/WebSocket between TUI and app-server. This adds serialization overhead and complexity. Runie runs in a single process — keep it that way. Only add IPC if you need remote agent execution.

3. **BearerAuth as primary auth.** Codex uses OAuth Bearer tokens. Runie targets local CLI use — API keys and OAuth device flow are sufficient. Don't add complex auth middleware.

### From OpenCode — What NOT to copy

1. **Effect framework for error handling.** OpenCode uses `Effect.ts` for functional error handling. In Rust, `Result<T, E>` is already algebraic and zero-cost. Don't add a functional effect library — it fights the language.

2. **SolidJS desktop app.** OpenCode has a full desktop app (Electron-like). Runie is terminal-native. Don't add a GUI framework — ratatui is the correct abstraction.

3. **Raw ANSI terminal UI alongside GUI.** OpenCode maintains two UI layers (SolidJS + ANSI terminal). This doubles maintenance burden. Runie should commit fully to ratatui TUI.

4. **Per-agent permission rulesets in TypeScript.** OpenCode's rulesets are dynamic JS objects. Runie should use compile-time validated rules (Rust structs) for safety.

### From Pi — What NOT to copy

1. **String-based rendering.** Pi's `render(width) → string[]` is simple but breaks down for complex layouts, overlapping regions, and Unicode width. Runie's ratatui `Buffer` approach is correct — don't regress.

2. **JavaScript single-threaded async.** Pi's event loop is single-threaded; tool parallelism is cooperative. Rust has true OS threads — use `tokio::spawn` for CPU-bound tools, `tokio::join!` for I/O-bound parallelism.

3. **No sandboxing.** Pi relies on OS-level process isolation. Runie should add at least filesystem sandboxing (landlock on Linux, seatbelt on macOS, Windows ACLs) before executing arbitrary bash commands.

4. **Flat message array.** Pi stores messages in a flat array. Runie's `Session` tree with `parent_id` links (`session.rs:26-38`) enables branching conversations, plan exploration, and undo — keep the tree.

### From Runie itself — What to fix

1. **Monolithic AppState.** `AppState` has 20+ fields mixing UI, agent, and animation concerns. Split into `UiState`, `AgentState`, `AnimationState`.

2. **Mixed event channel.** `AgentEvent::PermissionDecision` shouldn't exist. Split into separate channels.

3. **Tool execution always sequential.** The `ToolExecutionMode::Parallel` variant is unused. Either implement it or remove it.

4. **In-memory only.** Sessions lost on crash. SQLite persistence is Phase 3 critical.

---

## 5. Implementation Roadmap

### Phase 1: Quick Wins (1-2 weeks)

1. **Complete CancellationToken migration** — Remove all `JoinHandle::abort()` calls. Use cooperative cancellation checks at turn boundaries. Already partially done; just finish the sweep.
   - *Files:* `crates/runie-cli/src/tui_run.rs`, `crates/runie-agent/src/loop_engine.rs`
   - *Risk:* Low

2. **Remove EventForwarder trait, use closure** — Simplify event emission. Replace `Arc<dyn EventForwarder>` with `impl Fn(AgentEvent) + Send + Sync`.
   - *Files:* `crates/runie-agent/src/loop_engine.rs`, `crates/runie-cli/src/tui_run.rs`
   - *Risk:* Low

3. **Add `request_render()` to Tui** — Make render throttling a first-class TUI method, not a runtime hack.
   - *Files:* `crates/runie-tui/src/tui.rs`, `crates/runie-cli/src/tui_run.rs`
   - *Risk:* Low

### Phase 2: Medium Effort (2-4 weeks)

1. **Create `Agent` struct with owned state** — Wrap state, queues, active run. Move state reduction from TUI to Agent. Add `steer()` / `follow_up()` / `abort()` / `wait_for_idle()`.
   - *Files:* `crates/runie-agent/src/lib.rs` (new), `crates/runie-agent/src/state.rs`, `crates/runie-tui/src/tui/state.rs`
   - *Risk:* Medium — touches TUI update reducer

2. **Implement parallel tool execution** — Use `FuturesUnordered` for tool calls. Respect `ToolExecutionMode`.
   - *Files:* `crates/runie-agent/src/loop_engine.rs`
   - *Risk:* Medium — needs ordering guarantees in message history

3. **Add focused-component input routing** — Add `handle_input` to `Component` trait. Replace mode-based fallback with component-first routing.
   - *Files:* `crates/runie-tui/src/components/component.rs`, `crates/runie-tui/src/tui/events.rs`
   - *Risk:* Medium — changes all input handling

### Phase 3: Major Refactor (4-8 weeks)

1. **Add SQLite persistence layer** — New `runie-persistence` crate. `SessionStore` trait with SQLite impl. Save on exit, load on resume.
   - *Files:* New `crates/runie-persistence/`, `crates/runie-agent/src/state.rs`
   - *Risk:* High — data migration, schema evolution

2. **Implement MCP client integration** — New `runie-mcp` crate. Map MCP tools to `runie_core::Tool`.
   - *Files:* New `crates/runie-mcp/`
   - *Risk:* High — external protocol dependency

3. **Add overlay stack + Container layout** — Replace flat `TuiMode` enum with `Vec<Overlay>` stack. Implement `Container` that composes `Box<dyn Component>`.
   - *Files:* `crates/runie-tui/src/tui.rs`, `crates/runie-tui/src/components/component.rs`
   - *Risk:* High — rewrites rendering and input routing

---

## Appendix: Cross-Reference Mapping

| Pattern | Codex | OpenCode | Pi | Runie Target |
|---------|-------|----------|-----|--------------|
| Hook lifecycle | `session_start`, `pre_tool_use`, `post_tool_use` | Per-agent rulesets | `beforeToolCall`, `afterToolCall` | Expand `Hook` trait |
| State ownership | ThreadManager | InstanceState | `Agent` class | New `Agent` struct |
| Event emission | gRPC streaming | IPC messages | `emit()` callback | Closure-based |
| Cancellation | Task kill | Signal | `AbortSignal` | `CancellationToken` |
| Tool execution | Sequential (MCP) | Parallel (synthetic) | Seq/parallel modes | Implement parallel |
| Context pruning | Skills injection | Token tracking | `transformContext` | Hook + compactor |
| Persistence | SQLite (sqlx) | SQLite (Drizzle) | YAML files | SQLite (sqlx) |
| TUI rendering | ratatui buffer | SolidJS + ANSI | `string[]` lines | Keep ratatui |
| Provider auth | BearerAuth | API key + OAuth | OAuth device flow | Add OAuth |
| Sandboxing | landlock + Windows | Permission rules | None | Add landlock |

---

*End of document.*
