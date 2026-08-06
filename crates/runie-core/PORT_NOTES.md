# `runie-core` — port notes

Differences from `@earendil-works/pi-agent-core`:

| TS feature              | Rust port                                                         |
|-------------------------|-------------------------------------------------------------------|
| Declaration merging     | `trait AgentMessageExt` + `AgentMessage::Custom(Box<dyn …>)`       |
| Provider implementations| `trait StreamFn` (no built-in providers; adapter layer required)   |
| Callback registration   | `SubscriberRegistry` + `EventBus::broadcast`                       |
| TypeBox schemas         | Plain `serde_json::Value` for `Tool.parameters`                    |
| `AbortSignal`           | `tokio_util::sync::CancellationToken`                             |
| Per-tool execution mode | `AgentTool::execution_mode()` -> `Option<ToolExecutionMode>`       |
| Custom messages on wire | Opaque (serialized as `null`)                                     |

## Barrier semantics (preserved)

- `subscribe()` listeners are awaited in registration order.
- `agent_end` is the settlement barrier; `waitForIdle()` and `prompt()`
  return only after `agent_end` listeners resolve.
- `message_end` (assistant) is a barrier before tool preflight begins.

## What is NOT ported

- Provider implementations (Anthropic, OpenAI, Bedrock, Google, etc.).
  Supply a `StreamFn` adapter at the integration boundary.
- Harness: compaction, session persistence, skills, prompt templates.
- `node:sqlite` storage adapter.
- `streamProxy` browser-side streaming path.

## Status (2026-08-06)

The core event, state, queue, tool-dispatch, hook, and provider-replay
contracts are implemented and covered by local tests. `AgentStateActor` also
owns workflow lifecycle snapshots used by the TUI projection. The remaining
port boundary is deliberate: pi's provider catalog, OAuth, session/harness,
skills, compaction, and browser proxy packages are not part of `runie-core`.
Parity work and any future boundary expansion are tracked in `tasks/p30` and
the owning p01–p12 tasks.
