# p09 — Core tools: `AgentTool.prepareArguments` + `validateToolArguments` parity

**Parity target:** pi tool argument preparation + validation.

## Pi reference

`~/Code/agents/pi/packages/agent/src/agent-loop.ts`
- `prepareToolCallArguments` (line 586): if no `tool.prepareArguments`, return the toolCall unchanged. Else `prepared = tool.prepareArguments(toolCall.arguments)`; if `prepared === toolCall.arguments` return unchanged, else `{...toolCall, arguments: prepared}`.
- `validateToolArguments` — `~/Code/agents/pi/packages/ai/src/utils/validation.ts:278`: `structuredClone(args)` → `Value.Convert(parameters, args)` when a TypeBox schema → cached `Compile` validator → `coerceWithJsonSchema` (JSON-schema coercion: number/integer from string/bool/null, boolean from string/number, string from number/bool/null, null from `""`/0/false, object/array recursion, allOf/anyOf/oneOf union handling) → `validator.Check`. On failure throws `Validation failed for tool "<name>":\n<errors>\n\nReceived arguments:\n<JSON>` where each error is `  - <path>: <message>` with `.`-separators, `root` for empty.
- `validateToolCall` (line 263) throws `Tool "<name>" not found`.

## Current runie state (updated 2026-08-06)

`~/Code/GitHub/runie-tests/runie/crates/runie-core/src/types.rs`
- `AgentTool` exposes optional `prepare_arguments` and `parameters` hooks; the
  executor applies preparation before validation and execution.
- The executor now applies the common Pi JSON-schema coercions for object,
  array, string, number/integer, boolean, and null fields, checks required
  object properties, and emits Pi's structured validation header. Custom
  `validate_arguments` remains available for constraints beyond this portable
  schema subset.

## Adapt to runie

1. Add to `AgentTool`:
   ```rust
   fn prepare_arguments(&self, _args: &serde_json::Value) -> Option<serde_json::Value> { None }
   ```
   Applied in `execute_sequential`/`execute_parallel` per call (mirror `prepareToolCallArguments`).
2. Align `validate_arguments` error format with pi: `Validation failed for tool "<name>":` + `  - <path>: <message>` lines + `Received arguments:` JSON. Implement the JSON-schema coercion (number/boolean/string/null coercions + allOf/anyOf/oneOf) in a helper `coerce_json_schema(parameters, args)`.
3. `validate_tool_call`: unknown tool → `Tool "<name>" not found` (already handled in `prepareToolCall` equivalence).

## Progress

- **Portable schema boundary (2026-08-06):** `AgentTool::parameters()` is a
  JSON-schema equivalent of Pi's TypeBox parameters. `schema_rec` replay
  coverage proves string-to-integer and number-to-boolean coercion through the
  real loop, tool actor, and result event. Full TypeBox union semantics remain
  a documented follow-up because the Rust trait intentionally avoids a new
  schema compiler dependency.

## State machine / variants

Per-call prepare/validate pipeline:
```
toolCall → prepareArguments? (replace args if changed)
        → validate (coerce + check) → Ok | Err(Validation failed for tool "<name>": ...)
        → missing tool → Err("Tool \"<name>\" not found")
```
The `prepare_arguments` return variants: `None` (unchanged) | `Some(new_args)` (replace).

## Acceptance

- Unit tests: `prepare_arguments` replacement; JSON-schema coercion (string→number, bool, null); the exact pi error string format.
- `cargo test -p runie-core` green.
