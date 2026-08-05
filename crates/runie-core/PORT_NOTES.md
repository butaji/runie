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
  Reuse `runie-provider` or write a `StreamFn` adapter.
- Harness: compaction, session persistence, skills, prompt templates.
- `node:sqlite` storage adapter.
- `streamProxy` browser-side streaming path.

## Status

Steps 01-13 done. Step 14 in progress. Behavioural coverage focuses on the
README event-sequence contract; more scenarios are added as the crate matures.