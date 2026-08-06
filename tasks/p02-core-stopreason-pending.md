# p02 — Core types: add `StopReason::Pending` and reconcile stop-reason variants

**Parity target:** pi-agent-core stop-reason set.

## Pi reference

- `StopReason` — `~/Code/agents/pi/packages/ai/src/types.ts:391`
  ```ts
  export type StopReason = "pending" | "stop" | "length" | "toolUse" | "error" | "aborted";
  ```
- `"pending"` is the initial `stopReason` of the proxy-constructed streaming partial — `~/Code/agents/pi/packages/agent/src/proxy.ts:124` (partial init `{ role:"assistant", stopReason:"pending", ... }`). It is **never** seen as a final stop by the loop; it marks an in-progress assistant message.
- `"length"` triggers the truncated-tool-call guard — `agent-loop.ts:211`.
- `"error"` / `"aborted"` short-circuit the loop — `agent-loop.ts:196`.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/types.rs:60`
```rust
pub enum StopReason { Stop, ToolUse, MaxTokens, Error, Aborted }
```
- Has all except `Pending`. `MaxTokens` maps to pi `"length"`; serde is `rename_all="snake_case"`.

## Adapt to runie

Add the `Pending` variant:
```rust
pub enum StopReason {
    Stop,        // "stop"
    ToolUse,     // "toolUse"
    MaxTokens,   // "length"
    Error,       // "error"
    Aborted,     // "aborted"
    Pending,     // "pending"
}
```
Reconcile serde: pi uses camelCase `"toolUse"`. runie uses `snake_case` → `"tool_use"`. For wire/JSON parity with pi traces, either switch to `#[serde(rename_all="camelCase")]` with explicit `"tool_use"` mapping, or add a `#[serde(rename = "toolUse")]` on the variant. **Decision:** match pi's exact wire strings (`"toolUse"`, `"length"`, `"pending"`, `"stop"`, `"error"`, `"aborted"`) so replay traces and serialized state line up 1:1. Update any existing YAML fixtures that use `tool_use`/`max_tokens` accordingly.

## State machine / variants

- `Pending` is a **transient** value: a streaming partial holds it until `done`/`error` replaces it. The loop's `apply_event` (driver.rs) must not treat `Pending` as a terminal stop — only `Done{stop_reason}`/`Error` set a final value.
- `max_tokens` (`MaxTokens`) and `error`/`aborted` retain their existing loop behaviors (see p05 and p11).

## Acceptance

- Serde round-trip test covers all six variants with pi's exact wire strings.
- `cargo test -p runie-core` green; replay fixtures re-recorded if wire strings changed.

## Progress

- **Done (2026-08-05):** Added `Pending` and explicit pi wire names for all six
  stop reasons, with a serde round-trip test for the exact values.
