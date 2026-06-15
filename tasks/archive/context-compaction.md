# Context Compaction + Message Metadata

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: event-bus-jsonl-persistence, llm-event-normalization
**Blocks**: (none)

## Description

Runie currently keeps the full message history in `SessionState::messages`.
Long sessions eventually hit context-window limits. Research from Goose
(`MessageMetadata { user_visible, agent_visible }`), gptme (`pinned`, `hide`,
`quiet`, `ephemeral_ttl`), Aider (`done_messages` vs `cur_messages`), and
OpenHarness (`auto_compact_if_needed`) shows how to manage this.

This task adds message metadata, token-threshold auto-compaction, and a
summarization step that emits compacted events.

## Acceptance Criteria

- [ ] `ChatMessage` extended with metadata:
  ```rust
  pub struct MessageMetadata {
      pub pinned: bool,
      pub hidden_from_user: bool, // still sent to model
      pub ephemeral: bool,        // omitted from persistence
      pub compacted: bool,        // this message is a summary
  }
  ```
- [ ] `estimate_tokens` function is model-aware (uses `ModelCapabilities` max
  context tokens).
- [ ] `ContextCompactor` service:
  - Triggered before each LLM turn if estimated tokens exceed threshold.
  - Protects pinned and in-flight tool messages.
  - Summarizes oldest non-pinned messages into a `CompactedContext` event.
- [ ] Compaction progress emitted as transient `CompactProgress` events so the
  UI can show "Compacting conversation memory…".
- [ ] Manual `/compact` slash command triggers compaction immediately.
- [ ] Compacted context is stored as a durable `ContextCompacted` event in
  JSONL.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `compactor_triggers_above_threshold` — token estimate > 80% of context
  triggers compaction.
- [ ] `pinned_messages_not_compacted` — pinned messages remain intact.
- [ ] `compacted_event_replaces_old_messages` — summary event is generated.
- [ ] `ephemeral_message_not_persisted` — ephemeral event is not written to
  JSONL.

### Layer 2 — Event Handling
- [ ] `compact_command_emits_compaction_event` — `/compact` triggers
  `ContextCompacted`.
- [ ] `compact_progress_event_shows_phase` — phases emitted.

### Layer 3 — Rendering
- [ ] `compacted_message_renders_summary_badge` — UI shows a collapsed
  summary line.

## Notes

**Compaction strategy (MVP):**
- First pass: truncate oversized tool outputs and code blocks.
- Second pass: summarize the oldest 50% of non-pinned messages using the
  current provider (or a cheap fallback model if available).

**Files touched:**
- `crates/runie-core/src/message.rs`
- `crates/runie-core/src/context_compactor.rs` (new)
- `crates/runie-core/src/session_store.rs` (filter ephemeral)
- `crates/runie-agent/src/turn.rs` (trigger compaction)

**Out of scope:**
- Full/micro compaction tiers (Kimi Code style).
- Embedding-based memory retrieval.
