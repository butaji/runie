# Round 5 — Synthesis & Execution Roadmap

## Highest-impact, lowest-effort actions

1. **Stop flattening provider errors** — return typed `ProviderError`/`ModelError` and carry them into `Event`.
2. **Fix `temp_home()` isolation** — eliminates flaky HOME pollution across tests.
3. **Gate test support with `#[cfg(test)]`** — shrinks/safer production binary.
4. **Add tracing to `runie-provider`** and instrument actor handlers — makes async/event flow observable.
5. **Feature-gate heavy `runie-core` subsystems** — improves compile times and binary size.

## Recommended order

1. **Error handling quick wins**
   - `stop-flattening-provider-errors-into-strings`
   - `replace-production-expect-panics-with-result-propagation`
2. **Testing hygiene**
   - `fix-env-lock-isolation-and-remove-duplicates`
   - `gate-test-support-with-cfg-test`
   - `add-in-memory-backends-for-unit-tests`
3. **Telemetry**
   - `add-tracing-to-runie-provider`
   - `instrument-actor-handlers-with-tracing`
   - `replace-eprintln-println-with-tracing`
4. **Crate cleanup**
   - `feature-gate-heavy-runie-core-subsystems`
   - `remove-unused-workspace-dependencies`
   - `fix-unix-only-dependencies-in-runie-core`
   - `add-features-to-runie-provider`
5. **Deeper refactor**
   - `restructure-runieerror-with-typed-variants`
   - `break-runie-testing-dev-dependency-cycle`
   - `add-json-file-logging-for-tui`
   - `eliminate-real-sleeps-in-provider-tests`
   - `add-parameterized-tests-with-test-case`

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Execute roadmap | `tasks/execute-third-pass-architecture-review-roadmap.md` | **new** |
