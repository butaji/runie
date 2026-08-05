# p03 — Core types: Model + Usage field parity

**Parity target:** pi-ai `Model` and `Usage` shapes.

## Pi reference

- `Model` — `~/Code/agents/pi/packages/ai/src/types.ts:761`
  ```ts
  { id; name; api; provider; baseUrl; reasoning;
    thinkingLevelMap?; input: ("text"|"image")[]; cost;
    contextWindow; maxTokens; headers?; compat? }
  ```
- `Usage` — `types.ts:368`
  ```ts
  { input; output; cacheRead; cacheWrite; cacheWrite1h?; reasoning?;
    totalTokens; cost:{ input; output; cacheRead; cacheWrite; total } }
  ```

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/types.rs`
- `Model` (types.rs:280): `id, name, api, provider, base_url, reasoning, context_window, max_tokens`. **Missing:** `input` types, `cost`, `headers`, `compat`, `thinking_level_map`.
- `Usage` (types.rs:143): `input, output, cache_read, cache_write, total_tokens, cost`. **Missing:** `cache_write_1h`, `reasoning`.

## Adapt to runie

```rust
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: String,
    pub provider: String,
    #[serde(default)] pub base_url: String,
    #[serde(default)] pub reasoning: bool,
    #[serde(default)] pub thinking_level_map: Option<ThinkingLevelMap>, // pi: thinkingLevelMap?
    #[serde(default)] pub input: Vec<InputKind>,     // pi: input ("text"|"image")
    #[serde(default)] pub cost: CostBreakdown,       // pi: cost
    #[serde(default)] pub context_window: u64,       // pi: contextWindow
    #[serde(default)] pub max_tokens: u64,           // pi: maxTokens
    #[serde(default)] pub headers: HashMap<String,String>, // pi: headers?
}
```
`InputKind = Text | Image` (serde `"text" | "image"`). `ThinkingLevelMap` = budgets per `ThinkingLevel` (reuse `ThinkingBudgets`, types.rs:305).

```rust
pub struct Usage {
    pub input: u64, pub output: u64,
    pub cache_read: u64, pub cache_write: u64,
    #[serde(default)] pub cache_write_1h: u64,   // pi: cacheWrite1h?
    #[serde(default)] pub reasoning: u64,         // pi: reasoning?
    pub total_tokens: u64,
    #[serde(default)] pub cost: CostBreakdown,
}
```

## State machine / variants

- `InputKind` is a two-variant enum (`Text | Image`); `Model` is a plain data struct.
- No behavioral state machine here; the fields feed the provider (`provider/replay.rs`, `provider/http.rs`) and the loop's `AssistantMessage.usage`.

## Acceptance

- Serde round-trip tests assert exact JSON key names match pi (`cacheWrite1h`, `thinkingLevelMap`, camelCase).
- `default_convert_to_llm` and provider paths carry `usage` through `Done{usage}` into `AssistantMessage.usage` (ties into p01).
- `cargo test -p runie-core` green.