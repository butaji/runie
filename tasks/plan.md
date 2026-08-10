# Runie current plan

Updated: 2026-08-09

## Product direction

Runie is an event-driven Rust TUI with a Pi-compatible core and a Grok-shaped
presentation. Domain state is actor-owned; the UI consumes immutable
projections. Declarative schemas and macros describe repetitive domain data,
but async orchestration and business rules remain ordinary Rust.

## Workstreams

### 1. Declarative command and dialog surface

Status: implemented; keep extending through the shared registry.

- Keep command metadata, slash names, descriptions, argument schemas, handlers,
  and palette rows in one typed registry.
- Keep every overlay represented by the typed dialog DSL and `DialogStack`.
- Parameterized commands open forms; `Esc` pops one dialog level at a time.
- Files, models, sessions, skills, and commands should use the generic picker
  and form primitives.
- Acceptance: command registry coverage test, nested-form replay tests, and
  live 120×36 palette capture.

### 2. Pi/core parity

Status: active.

- Finish remaining provider transport options and session/lane persistence
  boundaries.
- Audit every observable mutable state transition for an owning actor event.
- Preserve streaming, retry, abort, tool identity, and lifecycle semantics.
- Acceptance: core replay suites, integration tests, and no direct cross-actor
  mutation.

### 3. TUI parity

Status: active.

- Maintain source-backed component contracts under `parity/tui/`.
- Close remaining transcript, waiting, tool-card, resize, theme, and animation
  gaps using deterministic replay fixtures.
- Compare geometry and cell styling at the documented terminal sizes.
- Acceptance: fixture snapshots, cell-level assertions, and fresh tmux captures
  when behavior depends on the live Grok surface.

### 4. Architecture and maintainability

Status: ongoing.

- Keep all `*.rs` files under 500 lines, functions under 40 lines, and
  complexity at or below 10.
- Prefer data declarations plus small typed macros for repetitive metadata,
  dispatch glue, dialog schemas, and replay fixtures. Current reusable macros
  include `component_specs!`, `declare_reducer_actor!`, and `event_trace!`.
- Do not hide actor lifecycle, error handling, provider streaming, or complex
  reducers inside macros.
- Acceptance: `target/debug/lint-check` reports clean and `cargo check
  --workspace` passes.

## Verification command

```sh
cargo fmt --all
cargo test --workspace
cargo build -p lint-check -q
target/debug/lint-check
cargo check --workspace --quiet
```

No task is complete from a green unit test alone; the relevant runtime,
replay, or capture evidence must also be recorded in `findings.md` or the
component documentation.
