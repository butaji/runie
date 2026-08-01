# Pi-Replication: Agentic Layer & Model Interaction

Implementation plan to make Runie's agentic layer and model interaction replicate
`~/Code/agents/pi` (the `@earendil-works/pi-agent-core` / `pi-ai` / `pi-coding-agent`
monorepo). Port the *architecture and contracts* — not pi's TUI (Ink), not its
TypeScript/TypeBox stack, not its vendor-SDK breadth.

## Research Summary

Pi's layering (identical to Runie's crate split, so this is a feature port):

```
pi-ai              ← Runie runie-provider   (providers, streaming, auth, retry)
pi-agent-core      ← Runie runie-agent      (agent loop, harness, compaction, skills)
pi-coding-agent    ← Runie core+cli+tui     (session, system prompt, modes)
```

Distinctive pi concepts to copy (from `packages/agent`, `packages/ai`,
`packages/coding-agent/src/core`):

1. **StreamFn contract** — the model boundary `(model, context, options) => EventStream`
   **never throws**. Failures are encoded as a final stream event with
   `stopReason: "error" | "aborted"`. Auth/setup runs *lazily inside the stream*
   (`lazyStream`) so even a missing API key terminates as an error event, not a thrown
   setup error.
2. **runLoop shape** (`agent-loop.ts`) — outer loop for follow-up/steering messages,
   inner loop for tool calls; `AgentLoopConfig` hooks: `prepareNextTurn` (model/thinking
   swap), `shouldStopAfterTurn`, `transformContext` (compaction hook point between loop
   and provider), `getSteeringMessages`/`getFollowUpMessages` (two drain modes),
   `beforeToolCall` (block) / `afterToolCall` (patch result incl. `terminate`),
   `getApiKey` (per-request key resolution for expiring OAuth tokens).
3. **Tool execution** — parallel by default, per-tool `executionMode: "sequential"`
   override; preflight all → execute concurrently → results emitted in assistant
   source order; batch `terminate` skips the follow-up LLM call; length-truncated
   messages **fail every tool call** (args may be silently truncated).
4. **Post-run decision chain** (`agent-session.ts::_handlePostAgentRun`) — outside the
   loop, in order: **retry** (retryable error → abortable exponential backoff → remove
   error message from agent state → `continue()`), **compaction** (overflow → compact +
   auto-retry exactly once; threshold → compact, no auto-retry), **drain** queued
   messages.
5. **Compaction v2** (`harness/compaction/compaction.ts`) — real usage accounting from
   last assistant message + chars/4 heuristic; turn-safe cut points (never mid-toolResult,
   split-turn prefix summarized separately); structured summarization
   (Goal / Constraints / Progress / Key Decisions / Next Steps / Critical Context) with an
   incremental *update* prompt; `reserveTokens=16384`, `keepRecentTokens=20000`,
   `maxTokens = 0.8 × reserveTokens`; read/write/edit path args tracked as
   `<read-files>` / `<modified-files>` metadata carried across compactions.
6. **Session as append-only tree** — entries `{id, parentId, timestamp}`, `leaf`
   pointer, `branch_summary` entries → in-place `/fork`, `/resume`, `/tree`; JSONL
   store; writes batched to turn boundaries (`save_point`).
7. **Provider layer** (`packages/ai`) — per-API **compat flags** on the model catalog
   (thinking format, strict tools, cache headers, session affinity) instead of
   provider-specific hacks; **CredentialStore** with lock-serialized OAuth refresh
   (refresh runs inside the store `modify` lock so concurrent requests can't
   double-refresh a rotated token); regex error classifier (transient vs
   quota/billing); partial-JSON tool-call streaming with a salvage parser.

## Current State (Runie)

| Area | Runie today | pi target |
|------|-------------|-----------|
| Stream errors | `Err` propagates (`turn/mod.rs:193`); provider build/auth fails before streaming | failures = terminal stream event; lazy auth inside stream |
| StopReason | `Length`/`ContentFilter` ignored beyond `Finish` | `Length` fails all tool calls in the message |
| Loop | fixed `for _ in 0..max_iterations` (`turn/mod.rs:147`) | inner/outer loop + 8 config hooks |
| Tool execution | strictly sequential (`turn/tools.rs`) | parallel default, per-tool sequential override |
| Retry | provider-level only, whole-request before content (`provider/retry.rs`) | + agent-level post-run retry with `continue()` |
| Compaction | drop oldest non-pinned + structural truncation (`core/model/compaction.rs`) | usage-accounted, turn-safe, LLM summarization, incremental update |
| Session | flat list (SessionActor) | tree with `parentId`/`leaf`/`branch_summary`, batched writes |
| Auth | env→dotenv→keyring→config resolver | + OAuth device-code + refresh-under-lock + per-request `getApiKey` |
| Provider quirks | MiniMax `ContentFilter` hacks (`provider/openai/protocol.rs`) | per-API `compat` flags on `ModelMeta` |
| System prompt | static string concat (`turn/mod.rs:278`) | structured: tool snippets + project instructions + skills XML + cwd |
| Skills | `core/harness_skills/` + `run_agent_turn_with_skills` | + frontmatter validation, `disable-model-invocation`, template engine |

## Phase 0 — Stream Contract: Errors Become Events

**Goal:** the provider stream is infallible from the loop's perspective; every
terminal state (success, error, abort, length) is a final event.

### 0.1 Lazy build/auth inside the stream

- `crates/runie-provider/src/lib.rs::build_provider` + `factory.rs`: defer credential
  resolution and request construction into the stream body. On failure, emit
  `ProviderEvent::Error(ModelError::…, terminal: true)` then `Done`, never `Err`.
  Mirrors pi's `lazyStream` (`packages/ai/src/models.ts`).
- `crates/runie-provider/src/openai/stream.rs`: same for the OpenAI-compatible path.

### 0.2 Infallible `stream_response`

- `crates/runie-agent/src/stream_response.rs`: return `StreamedResponse` that always
  terminates; add `terminal_reason: Option<StopReason>` to it.
- `crates/runie-agent/src/turn/mod.rs:193`: delete the `Err(e)` early-return branch —
  the loop proceeds with whatever partial state the stream produced, then the post-run
  chain (Phase 1.3) decides retry/compaction.

### 0.3 Act on `StopReason::Length`

- `crates/runie-agent/src/turn/mod.rs::run_agent_iteration`: when
  `terminal_reason == Length` and the message contains tool calls, mark every
  `ParsedToolCall` as failed (error tool-result visible to the model), do not execute.
  Port of pi's `failToolCallsFromTruncatedMessage` (`agent-loop.ts`).

### 0.4 Abort normalization

- Provider and loop both normalize abort → final `error/aborted` event, including
  mid-backoff (Phase 1.3). No thrown `Cancelled` leaks to the actor.

### Tests (Phase 0)

- `crates/runie-provider/src/tests.rs`: stream fixture that errors mid-frame →
  `terminal_reason == Some(Error)`, loop continues cleanly.
- `crates/runie-agent/src/turn/tests.rs`: truncated assistant message with tool calls →
  tool calls failed, no execution; abort token fired mid-stream → aborted terminal.
- Replay e2e: add a fixture under `crates/runie-provider/src/fixtures/` exercising
  error-mid-stream; run via `RUNIE_REPLAY_FIXTURES`.

## Phase 1 — Agent Loop Parity

### 1.1 Loop shape + config hooks

New module `crates/runie-agent/src/loop/`:

```rust
// loop/config.rs
pub struct AgentLoopConfig {
    pub tool_execution: ToolExecution,          // Parallel | Sequential (default Parallel)
    pub max_iterations: usize,                  // replaces today's `max_iterations` param
    pub prepare_next_turn: Option<Arc<dyn Fn(PrepareNextTurnCtx) -> NextTurnPlan + Send + Sync>>,
    pub should_stop_after_turn: Option<Arc<dyn Fn(&TurnStats) -> bool + Send + Sync>>,
    pub transform_context: Option<Arc<dyn Fn(Vec<ChatMessage>) -> Vec<ChatMessage> + Send + Sync>>,
    pub get_steering_messages: Option<SteeringQueue>,   // QueueMode::All | OneAtATime
    pub get_follow_up_messages: Option<SteeringQueue>,
    pub get_api_key: Option<Arc<dyn Fn(&str) -> Option<SecretString> + Send + Sync>>,
    pub before_tool_call: Option<Arc<dyn Fn(&ParsedToolCall) -> ToolCallVerdict + Send + Sync>>,
    pub after_tool_call: Option<Arc<dyn Fn(&mut ToolOutput) + Send + Sync>>,
}
pub enum NextTurnPlan { Continue, SwapModel { model: String, thinking_level: u8 }, End }
pub enum ToolCallVerdict { Allow, Block { reason: String } }
```

- `crates/runie-agent/src/loop/agent_loop.rs`: port `runLoop`/`streamAssistantResponse`
  control flow; `transform_context` applied between message build and provider call.
- `crates/runie-agent/src/turn/mod.rs`: `run_agent_turn_with_skills` becomes a thin
  adapter constructing a default `AgentLoopConfig`; keep the skills hooks
  (`on_turn_start`/`on_turn_end`) as `before/after` adapters.
- `crates/runie-agent/src/actor/handlers.rs`: wire `prepare_next_turn` (model/thinking
  swap) and `should_stop_after_turn` so the actor can end or re-enter a turn.

### 1.2 Parallel tool execution

- `crates/runie-agent/src/turn/tools.rs`: preflight all calls (permission gate,
  arg validation, `before_tool_call`) sequentially → `tokio::join_all` the executable
  bodies → emit `tool_execution_end` in completion order but append tool-result
  messages in assistant source order (pi's invariant).
- `crates/runie-core/src/tool/schema.rs` (`ToolDef`): add `execution_mode: ExecutionMode`
  (`Parallel | Sequential`); if **any** tool in the batch declares `Sequential`, the
  whole batch runs sequentially (pi rule).
- `crates/runie-agent/src/tool_runner.rs`: respect per-tool timeouts under join_all
  (keep the 30s timeout per call).
- Batch `terminate` semantics: if every tool result sets `terminate`, skip the follow-up
  iteration and end the turn.

### 1.3 Post-run decision chain

New `crates/runie-agent/src/post_run.rs`, called by the actor after a turn completes
(instead of immediately emitting `Done`):

```
post_run(stats):
  1. Retry      — terminal_reason == Error && is_retryable(ModelError)
                     → abortable backoff (base 100ms × 2^(n−1), cap 30s)
                     → remove the error message from agent state (keep in session history)
                     → continue() (requires last msg is user/toolResult — enforce)
  2. Compaction — overflow (usage/contextWindow, or ContextLength error)
                     → compact (Phase 2), drop error message, auto-retry exactly once
                     (guard: _overflowRecoveryAttempted, resets per user prompt)
                  threshold (tokens > contextWindow − reserveTokens)
                     → compact, no auto-retry
  3. Drain      — queued steering/follow-up messages from agent_end handlers → continue
```

- `crates/runie-agent/src/actor/handlers.rs`: `TurnComplete` ack now re-enters the loop
  per the chain; `Done` emitted only when the chain is exhausted.
- Move the retry classifier surface so provider and agent share one
  `is_retryable` (see Phase 3.3).

### Tests (Phase 1)

- `loop/` unit tests: parallel vs sequential batches, source-order result emission,
  `terminate` batch, block verdict, `transform_context` applied, steering queue
  drain modes.
- `post_run.rs` tests: retryable error → backoff+continue; overflow → compact +
  single auto-retry; non-retryable (quota) → no retry. No `sleep()` — inject a mock
  timer (tokio `time::pause` or a `DelayFn` seam).
- Actor e2e with `RUNIE_MOCK_SCRIPT`/replay fixtures: retry loop, stop-after-turn.

## Phase 2 — Compaction V2

Rewrite `crates/runie-core/src/model/compaction.rs` (keep `truncate_messages_structurally`
as a pre-pass). New design:

### 2.1 Token accounting

- `calculate_context_tokens(messages, last_usage)`: prefix counted from the last
  assistant message's real `usage.input_tokens`; tail messages via
  `estimate_tokens` (chars/4 heuristic, per-role). Extend
  `crates/runie-provider/src/provider_event.rs` `Usage` to carry input/cache tokens
  (already partially present) and thread it into `StreamedResponse`.

### 2.2 Turn-safe cut points

- `find_cut_point(messages, keep_recent_tokens=20000)`: walk from the end accumulating
  tokens; only cut at valid boundaries (user/assistant/bash/custom/compaction summary —
  never mid-toolResult); roll back to turn starts. If the cut splits a turn, summarize
  the turn prefix separately (`TURN_PREFIX_SUMMARIZATION_PROMPT`).

### 2.3 Summarization

- Prompts in `crates/runie-core/src/prompts.rs`:
  `SUMMARIZATION_SYSTEM_PROMPT` (Goal / Constraints / Progress Done-InProgress-Blocked /
  Key Decisions / Next Steps / Critical Context; "preserve exact file paths, function
  names, error messages") and `UPDATE_SUMMARIZATION_PROMPT` for incremental compaction
  when a previous summary exists.
- Runner: `crates/runie-agent/src/compaction.rs` — the summarization LLM call goes
  through the same provider with `cacheRetention: none` + a fresh session-id (isolated
  routing); `max_tokens = 0.8 × reserveTokens (16384)`; on failure fall back to the
  current drop-oldest behavior (never lose history silently).
- Result inserted as a `ChatMessage` with `metadata.origin = Compaction` (existing
  pattern) wrapped in `<summary>…</summary>` blocks per pi's `messages.ts` conversion.

### 2.4 File-op metadata

- Track `read_file`/`write_file`/`edit_file` path args per turn; append
  `<read-files>` / `<modified-files>` blocks to the summary; carry forward across
  compactions (pi carries it in entry `details`).

### Tests (Phase 2)

- Cut-point unit tests: mid-toolResult refusal, split-turn prefix summarization,
  pinned-message preservation.
- Summarization prompt contract test (snapshot the built prompt).
- Integration: mock provider that returns a fake summary → verify message list shape,
  `<summary>` block, auto-retry-on-overflow path from Phase 1.3.

## Phase 3 — Provider & Model Interaction Parity

### 3.1 Compat flags

- `crates/runie-core/src/provider/registry.rs` (`ModelMeta`): add
  `compat: ModelCompat` — flags for thinking format (field vs `<think>` tags), strict
  tools, cache headers, session affinity.
- `crates/runie-provider/src/openai/protocol.rs`: replace the MiniMax `ContentFilter`
  special-casing with per-`compat` dispatch (keep the existing chunk-boundary-safe
  machinery, gate it on the flag).

### 3.2 Auth: OAuth + refresh-under-lock + per-request key

- `crates/runie-provider/src/auth.rs` (new): `CredentialStore` with a per-provider
  `tokio::sync::Mutex`; `modify` is the only write path and OAuth refresh runs *inside*
  the lock (prevents double-refresh of a rotated token — pi's `CredentialStore.modify`).
- `ApiKeyAuth::resolve` semantics: `credential.key ?? env ?? config` (matches Runie's
  existing resolver order; add `check`/`login`).
- OAuth device-code flow: port the state machine from `packages/ai/src/auth/oauth/`
  (device-code + token refresh), wired into `runie login` (`crates/runie-cli`).
- Per-request `get_api_key`: thread `AgentLoopConfig.get_api_key` (Phase 1.1) through
  `AgentCommand` → `stream_response` → provider build, so a refreshed token is picked
  up mid-session without rebuilding the provider.

### 3.3 Retry wiring

- `crates/runie-provider/src/retry.rs`: wire the existing per-error-type `RetryPolicy`
  into the streaming path (currently used only in tests). Keep whole-request retry
  *before content* (no duplicate output); idle-timeout stays non-retryable.
- Export one shared `is_retryable(ModelError)` used by both provider retry and the
  Phase 1.3 post-run chain; ensure quota/billing/usage-limit are non-retryable
  (pi's classifier).

### 3.4 Partial-JSON salvage parser

- `crates/runie-agent/src/streaming_parser.rs`: replace/augment tool-call JSON
  accumulation with a salvage parser (pi's `parseStreamingJson`) so truncated or
  whitespace-split JSON args still yield a partial tool call (which then gets failed by
  the Phase 0.3 Length handling instead of silently dropped).

### Tests (Phase 3)

- Compat flag matrix: MiniMax fixture (tags), OpenAI fixture (native fields) — same
  `ProviderEvent` output.
- CredentialStore: concurrent `modify` serialization test (two refresh attempts → one
  network call, mock the refresh client).
- Retry: per-error-type counts honored; quota error not retried at agent layer.

## Phase 4 — Session, System Prompt, Skills, Persistence

### 4.1 Session as append-only tree (prereq for `/fork`)

- `crates/runie-core/src/session/` + SessionActor: migrate flat `Vec<ChatMessage>`
  persistence to entries `{id, parent_id, timestamp}` with `leaf` pointer and
  `branch_summary` entries (JSONL store, `append_tail` serialized). Keep the existing
  disk-replay late-subscriber catch-up.
- CLI/TUI: `/fork`, `/resume`, `/tree` (`crates/runie-cli`, `crates/runie-tui`).

### 4.2 Structured system prompt

- `crates/runie-core/src/prompts.rs::build_system_prompt`: compose role text +
  `Available tools:` with per-tool `prompt_snippet` (from `ToolDef`) + conditional
  `prompt_guidelines` (added only when the tool is active) + `<project_context>
  <project_instructions path=…>` blocks from an AGENTS.md/CLAUDE.md walk (respect
  existing `AGENTS.md` loading if present) + skills XML + cwd.
- `crates/runie-agent/src/turn/mod.rs::build_initial_messages`: use the new builder;
  keep read_only filtering.

### 4.3 Skills parity

- `crates/runie-core/src/harness_skills/`: add YAML frontmatter parsing/validation
  (`name`, `description`, `disable-model-invocation`), strict name check
  (lowercase `a-z0-9-`, ≤64 chars), and a template engine (`/name`, `$1`, `$@`,
  `$ARGUMENTS`, `${@:N:L}`) if absent today. System-prompt exposure: XML
  `<available_skills><skill><name/><description/><location/></skill></available_skills>`
  block + `<skill name location>` invocation wrapping.

### 4.4 Batched session writes

- SessionActor: queue mutations during a turn as pending writes; flush at `turn_end`
  (emit `save_point`) and `agent_end` — pi's `PendingSessionWrite` pattern. Reduces
  disk churn vs per-event writes.

### Tests (Phase 4)

- Tree session: fork → switch leaf → resume; branch_summary entry semantics.
- System prompt: snapshot tests for tool-snippet/guideline composition.
- Skills: frontmatter validation, template expansion (match existing
  `skills_commands` test conventions).

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `crates/runie-agent/src/loop/config.rs` | `AgentLoopConfig`, hook types |
| `crates/runie-agent/src/loop/agent_loop.rs` | inner/outer loop, streaming, hooks |
| `crates/runie-agent/src/post_run.rs` | retry → compaction → drain chain |
| `crates/runie-agent/src/compaction.rs` | summarization runner (Phase 2) |
| `crates/runie-provider/src/auth.rs` | CredentialStore, OAuth refresh-under-lock |
| `crates/runie-provider/src/auth/oauth.rs` | device-code flow (or under `auth/`) |

### Modified Files

| File | Changes |
|------|---------|
| `crates/runie-agent/src/turn/mod.rs` | adapter to loop module; Length handling; infallible stream; prompt builder |
| `crates/runie-agent/src/turn/tools.rs` | parallel execution, source-order results |
| `crates/runie-agent/src/stream_response.rs` | infallible terminal + `terminal_reason` |
| `crates/runie-agent/src/streaming_parser.rs` | salvage JSON parser |
| `crates/runie-agent/src/actor/handlers.rs` | post-run chain wiring, `prepare_next_turn` |
| `crates/runie-agent/src/tool_runner.rs` | per-tool timeouts under join_all |
| `crates/runie-agent/src/agent_command_builder.rs` | `get_api_key`, loop config fields |
| `crates/runie-core/src/tool/schema.rs` | `ToolDef.execution_mode`, prompt snippet/guidelines |
| `crates/runie-core/src/model/compaction.rs` | v2: usage accounting, turn-safe cuts, summarization |
| `crates/runie-core/src/model/state.rs` | usage threading, pending-write queue |
| `crates/runie-core/src/provider/registry.rs` | `ModelMeta.compat` |
| `crates/runie-core/src/prompts.rs` | summarization prompts, structured system prompt |
| `crates/runie-core/src/provider_event.rs` | `Usage` fields, terminal stop-reason plumbing |
| `crates/runie-core/src/session/` | tree entries, JSONL, batched writes |
| `crates/runie-core/src/harness_skills/` | frontmatter validation, templates |
| `crates/runie-provider/src/lib.rs` / `factory.rs` | lazy build/auth inside stream |
| `crates/runie-provider/src/openai/protocol.rs` | compat-flag dispatch (replace ContentFilter hacks) |
| `crates/runie-provider/src/retry.rs` | wire per-error-type `RetryPolicy` |
| `crates/runie-cli` | OAuth login flow, `/fork`/`/resume`/`/tree` |
| `crates/runie-tui` | fork/resume/tree commands (TUI changes need live tmux run) |

## Priority Order

1. **Phase 0** — stream contract (foundation; unblocks everything).
2. **Phase 1.1 + 1.2** — loop shape + parallel tools (immediate UX change).
3. **Phase 1.3** — post-run retry/compact/continue chain (biggest correctness win).
4. **Phase 2** — compaction v2 (summarization beats blind dropping).
5. **Phase 3** — compat flags, auth/OAuth, retry wiring.
6. **Phase 4** — session tree, system prompt, skills, batched writes (largest surface).

Out of scope (deliberately): multi-vendor SDK breadth (keep OpenAI-compatible funnel),
pi's TUI, permission engine rebuild.

## Verification

Per `AGENTS.md`: fast automatic tests per layer; **no `sleep()`** (use tokio
`time::pause` or injected delay seams); run layer 4 replay before push when touching
async/event logic.

```bash
# Unit + integration per crate
cargo test -p runie-provider -p runie-agent -p runie-core

# Layer 4: replay fixtures (retry, error-mid-stream, Length-truncation)
RUNIE_REPLAY_FIXTURES=crates/runie-provider/src/fixtures cargo test -p runie-tests --test replay_agent

# Parallel tools e2e via mock script
RUNIE_MOCK_SCRIPT=... cargo run -p runie-cli -- print "..."

# TUI (only after TUI-affecting changes, live terminal/tmux):
just tui --mock   # then exercise /fork /resume /tree, mid-turn steering

# Black-box replay (CLI/TUI wiring changes):
RUNIE_REPLAY_FIXTURES=... cargo test -p runie-tests
```
