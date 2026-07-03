# Round 4 — Dependency Graph & Crate Boundaries

## Findings

### 1. `runie-core` is a fat crate with no features

- `crates/runie-core/Cargo.toml` declares ~80 dependencies.
- Heavy/optional subsystems (MCP, keyring, git status, file watching, clipboard, markdown parsing, model catalog YAML) are always compiled.

### 2. Circular-ish dev dependency

- `runie-testing` depends on `runie-agent` and `runie-provider`.
- `runie-agent` and `runie-provider` dev-depend on `runie-testing`.

### 3. Portability / dependency oddities

- `crates/runie-core/Cargo.toml:87-94` places `async-trait`, `derive_builder`, `ractor`, `parking_lot`, and `uuid` under `target.'cfg(unix)'`. On non-Unix builds the crate will fail to compile.
- `crates/runie-core/Cargo.toml:57` and `crates/runie-core/Cargo.toml:96` list `tempfile` in both `[dependencies]` and `[dev-dependencies]`.
- `Cargo.toml:64` declares `tracing-appender` in the workspace but it is unused.

### 4. `runie-provider` has no optional features

- All providers (currently OpenAI-compatible + mock) are always built.

## Recommended changes

1. Introduce feature flags in `runie-core` for MCP, keyring, git, notify, clipboard, markdown YAML, model catalog.
2. Remove unused workspace dependencies (`tracing-appender`).
3. Fix Unix-only dependency placement and duplicate `tempfile`.
4. Move provider/agent-specific mocks out of `runie-testing` or make `runie-testing` dev-dependency-only to break the cycle.
5. Add `openai`, `mock` features to `runie-provider`.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Feature-gate runie-core | `tasks/feature-gate-heavy-runie-core-subsystems.md` | **new** |
| Remove unused workspace deps | `tasks/remove-unused-workspace-dependencies.md` | **new** |
| Fix unix-only deps and duplicate tempfile | `tasks/fix-unix-only-dependencies-in-runie-core.md` | **new** |
| Break testing dev-dependency cycle | `tasks/break-runie-testing-dev-dependency-cycle.md` | **new** |
| Feature-gate runie-provider | `tasks/add-features-to-runie-provider.md` | **new** |
