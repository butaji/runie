# Round 3 — Logging, Telemetry & Diagnostics

## Findings

### 1. `runie-provider` is silent

- Grep found **zero** `tracing` events or spans in `crates/runie-provider/src`.

### 2. Actor handlers are uninstrumented

- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-core/src/actors/provider/ractor_provider.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`

None have `#[tracing::instrument]` on their `handle` methods.

### 3. Unstructured / noisy output

- `crates/runie-core/src/tracing_init.rs:20-38` — only a plain fmt layer; no JSON.
- `crates/runie-core/src/tests/check_tiktoken_vals.rs:10-13`, `crates/runie-core/src/markdown/tests.rs:31-228` — use `println!`/`eprintln!` in tests.
- `crates/runie-core/src/subagents/mod.rs:86,90` — `eprintln!` in production code for template errors.

### 4. Metrics are a no-op

- `crates/runie-core/src/metrics.rs:25-27` installs a `NoopRecorder`; few counters/histograms are emitted.

### 5. No file logging for TUI

- TUI tracing goes to stdout/stderr, which can corrupt the terminal.

## Recommended changes

1. Add `tracing` spans/events to `runie-provider` around retries, validation, and SSE parsing.
2. Instrument actor message handlers with span fields for `turn_id`, `provider`, `model`.
3. Replace `eprintln!`/`println!` with `tracing::debug!` or assertions.
4. Add JSON file logging for `runie-tui` (with stdout/stderr suppressed while UI is active).
5. Either wire a metrics recorder behind a feature flag or document that metrics are intentionally disabled.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Add tracing to provider | `tasks/add-tracing-to-runie-provider.md` | **new** |
| Instrument actors | `tasks/instrument-actor-handlers-with-tracing.md` | **new** |
| Replace eprintln/println | `tasks/replace-eprintln-println-with-tracing.md` | **new** |
| JSON file logging for TUI | `tasks/add-json-file-logging-for-tui.md` | **new** |
