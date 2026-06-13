# Provider Normalization Behind `LLMEvent`

## Context

`runie-provider` currently returns `ResponseChunk { content: String }`. The
agent and TUI must parse provider-specific text (OpenAI deltas, Anthropic tool
blocks) from raw strings. This leaks provider details throughout the stack and
makes tool-call rendering brittle.

Goose, OpenCode, Gemini CLI, and OpenHarness all normalize provider streams to
a provider-agnostic event vocabulary before the rest of the app sees them.

## Decision

Introduce a single `LLMEvent` enum in `runie-core` and make every provider
implementation emit it:

```rust
pub enum LLMEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ToolCallStart { id, name },
    ToolCallInputDelta { id, delta },
    ToolCallEnd { id },
    Error(ProviderError),
    Usage { input_tokens, output_tokens },
    Finish { reason },
}
```

The `Provider` trait returns `Stream<Item = Result<LLMEvent>>`. Provider
adapters in `runie-provider` handle OpenAI, Anthropic, and any future backend.
The agent layer maps `LLMEvent` to `AgentEvent` for the bus.

## Consequences

- **Positive:** Tool calls are first-class objects, not text to parse.
- **Positive:** TUI can render streaming and tool states generically.
- **Positive:** Adding a new provider only requires one adapter.
- **Trade-off:** We keep a thin in-house provider layer instead of using
  `async-openai`, preserving support for non-OpenAI-shaped backends.
