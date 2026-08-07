# p03 — Core types: Model + Usage field parity

**Latest parity note (2026-08-05):** `Model.thinking_level_map` now matches
pi's provider-effort mapping (`off` through `max` to string values), while
numeric per-level token budgets remain in the separate `ThinkingBudgets` type.

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
- `Model` now carries `input`, `cost`, `headers`, `compat`, and
  `thinking_level_map` in addition to the base fields; camelCase serde and
  fractional pricing/compatibility round trips are tested.
- `Usage` now carries `cache_write_1h` and `reasoning`, with the full cost
  object and Pi-compatible wire keys covered by round-trip tests.

The former “Missing” list below is retained only as historical scope; it is
not current implementation debt.

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

## Progress

- **Fractional cost parity (2026-08-05):** `CostBreakdown` now uses floating
  point USD values like pi-ai instead of integer token-cost fields, with a
  serde regression asserting fractional input/output costs survive the wire
  round trip.
- **Model pricing/compat parity (2026-08-05):** Split model pricing into
  `ModelCost` with optional `ModelCostTier` entries and added the optional
  provider `compat` projection. Serde coverage pins fractional rates, the
  `inputTokensAbove` tier key, and compatibility data.
- **Wire-key parity (2026-08-05):** Model and usage/cost serializers now use
  pi-compatible camelCase keys (`baseUrl`, `thinkingLevelMap`, `cacheRead`,
  `cacheWrite1h`, `totalTokens`, and related fields), with round-trip tests
  asserting the wire names.
