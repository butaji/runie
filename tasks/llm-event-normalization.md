# Normalize All Providers Behind a Single `LLMEvent` Stream

**Status**: todo
**Milestone**: R3
**Category**: Providers & Models
**Priority**: P0

**Depends on**: event-bus-jsonl-persistence
**Blocks**: model-capability-flags

## Description

Currently `runie-provider` exposes a `Provider` trait that yields
`ResponseChunk { content }`. The agent layer and TUI see provider-specific
content as raw strings. Research of Goose, OpenCode, Gemini CLI, and
OpenHarness shows that the cleanest design is a provider-agnostic event
stream: `LLMEvent`.

This task normalizes every provider to emit the same event vocabulary:
text deltas, reasoning/thinking deltas, tool calls, tool-input deltas,
errors, usage, and finish reasons.

## Acceptance Criteria

- [ ] `crates/runie-core/src/llm_event.rs` defines a typed enum:
  ```rust
  pub enum LLMEvent {
      TextDelta(String),
      ThinkingDelta(String),
      ToolCallStart { id: String, name: String },
      ToolCallInputDelta { id: String, delta: String },
      ToolCallEnd { id: String },
      Error(ProviderError),
      Usage { input_tokens: usize, output_tokens: usize },
      Finish { reason: StopReason },
  }
  ```
- [ ] `runie-provider/src/lib.rs` `Provider` trait updated to:
  ```rust
  fn generate(&self, req: LLMRequest) -> Pin<Box<dyn Stream<Item = Result<LLMEvent>> + Send>>;
  ```
- [ ] `LLMRequest` value object unifies all provider inputs:
  ```rust
  pub struct LLMRequest {
      pub model: String,
      pub messages: Vec<Message>,
      pub tools: Vec<ToolSchema>,
      pub system_prompt: Option<String>,
      pub max_tokens: Option<usize>,
      pub thinking_level: ThinkingLevel,
  }
  ```
- [ ] `runie-provider/src/openai.rs` and `runie-provider/src/anthropic.rs`
  rewritten to emit `LLMEvent` instead of raw `ResponseChunk`.
- [ ] `runie-agent/src/turn.rs` consumes `LLMEvent` and emits matching
  `AgentEvent` variants to the bus.
- [ ] `runie-tui` no longer parses tool calls from raw assistant text; it
  receives `AgentEvent::ToolCallStart/End` from the bus.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `openai_provider_emits_text_delta` — mock server returns a streaming
  completion, provider yields `LLMEvent::TextDelta`.
- [ ] `anthropic_provider_emits_tool_call_start` — mock server returns a
  tool_use block, provider yields `LLMEvent::ToolCallStart`.
- [ ] `llm_request_serializes_round_trip` — `LLMRequest` → JSON → `LLMRequest`.

### Layer 2 — Event Handling
- [ ] `agent_turn_maps_llm_event_to_agent_event` — feed a sequence of
  `LLMEvent`s into `run_agent_turn` and assert the emitted `AgentEvent`s.

### Layer 3 — Rendering
- [ ] `tool_call_start_renders_inline_card` — TUI receives
  `AgentEvent::ToolCallStart` and renders a tool card.

## Notes

**Why not `async-openai`:**
- Evaluated via `crate-replacement-audit`. `async-openai` is a strong crate,
  but it locks us into OpenAI-shaped APIs. Runie supports 35+ providers with
  varying response shapes (Anthropic tool blocks, Ollama, Gemini, etc.).
  Keeping a thin in-house adapter behind `LLMEvent` preserves flexibility.

**Files touched:**
- `crates/runie-core/src/llm_event.rs` (new)
- `crates/runie-core/src/provider.rs` (update trait)
- `crates/runie-provider/src/lib.rs`
- `crates/runie-provider/src/openai.rs`
- `crates/runie-provider/src/anthropic.rs`
- `crates/runie-provider/src/mock.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/parser.rs` (tool parsing may become obsolete)

**Out of scope:**
- Provider failover / multi-provider routing.
- Structured output / JSON mode enforcement.
