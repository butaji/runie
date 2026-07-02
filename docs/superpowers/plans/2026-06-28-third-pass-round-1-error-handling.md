# Round 1 — Error Handling & Result Types

## Findings

### 1. Central `RunieError` is unused

- `crates/runie-core/src/error.rs:35-92` — `RunieError` wraps `anyhow::Error`; `RunieErrorKind` is defined but never connected.
- A grep shows **zero** production/test usage of `RunieError`/`RunieErrorKind` outside `error.rs`.

The wrapper adds no value today.

### 2. Typed provider errors are flattened into strings

- `crates/runie-core/src/provider_event.rs:111-116` — `From<anyhow::Error> for ModelError` stores only `e.to_string()`, losing typed variant and source chain.
- `crates/runie-core/src/event/from_provider_event.rs:29-32` — `ProviderEvent::Error` becomes `Event::Error { message: String }`, discarding `ModelError` structure.
- `crates/runie-provider/src/openai/stream.rs:147` — SSE errors are re-wrapped with `anyhow::anyhow!("{:?}", ...)`, throwing away `ProviderError`.

### 3. `ProviderError` clone/downcast is lossy

- `crates/runie-core/src/provider/provider_trait.rs:80-97` — `Clone` for `ProviderError::Source` formats the inner `anyhow::Error` to a string.
- `crates/runie-core/src/provider/provider_trait.rs:192-201` — `From<anyhow::Error>` downcasts back to `ProviderError`, but most call sites wrap typed errors with `anyhow!(...)` first, defeating the downcast.

### 4. Actors emit generic `Event::Error { message }`

- `crates/runie-core/src/actors/provider/ractor_provider.rs:57,79,96` — generic `anyhow!` strings for actor unavailability.
- `crates/runie-core/src/actors/config/handlers.rs:96,123,255,262,306,339` — config failures become plain error events.

### 5. Panic-prone production code

- `crates/runie-core/src/tool/shim/mod.rs:244` — `.unwrap()` on parser result.
- `crates/runie-core/src/model/compaction.rs:194,248` — `Regex::new(...).unwrap()` in non-test code.
- `crates/runie-core/src/session/tree.rs:151` — `.expect("session tree clone failed ...")`.
- `crates/runie-provider/src/openai/stream.rs:89` — `.expect("request builder is cloneable")` inside retry closure.

## Recommended changes

1. Either delete `RunieError` or turn it into a real central enum with `RunieErrorKind` variants.
2. Propagate `ProviderError`/`ModelError` without string conversion; add a structured `Event::ModelError { error: ModelError }` variant.
3. Remove the `anyhow!(typed_err)` wrapping at provider call sites so downcast works.
4. Introduce typed actor/ops error enums and convert them to events with an error-kind field.
5. Replace production `.unwrap()`/`.expect()` with `Result` propagation or `LazyLock` for regexes; document any remaining invariants.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Restructure `RunieError` | `tasks/restructure-runieerror-with-typed-variants.md` | **new** |
| Stop flattening provider errors | `tasks/stop-flattening-provider-errors-into-strings.md` | **new** |
| Replace panic-prone `unwrap`/`expect` | `tasks/replace-production-expect-panics-with-result-propagation.md` | **new** |
