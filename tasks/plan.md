# Runie current plan

Updated: 2026-08-10

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

- Finish remaining provider transport options and session/lane persistence; lane snapshots now expose strict sequence validation for replay/storage callers.
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

## Ranked remaining implementation queue

The reduction backlog is now backed by declarative tables for the major closed
vocabularies, so the next work is functional parity. Each item remains open
until its source change, event/replay tests, and live TUI evidence are recorded.

1. Provider-specific request adapters — `harness-04` and `harness-14`.
   MiniMax effort projection is now covered by a pure request-body regression;
   replay finish-reason mapping now preserves raw chat values with a
   compatibility-tested tool-call path; complete the finite provider
   payload/finish-reason conformance matrix,
   including unsupported-effort behavior and normalized failures.
2. Tool output and background lifecycle UX — `harness-01` and `harness-03`.
   `/jobs cancel all` now reduces every running job through the actor mailbox;
   `ToolCardSummary` now exposes bounded output line/byte/truncation facts;
   add richer renderer-neutral output cards and owned lifecycle controls,
   with failure/cancellation replay traces.
3. Model-aware context policy — `harness-05` (live usage boundary implemented).
   Recovery and `/context` now use the active model’s declared window;
   remaining work is richer compaction controls and threshold/recovery replay
   coverage for those controls.
4. Scheduler cancellation controls — `harness-08` (actor-owned metrics and `/jobs` projection implemented; richer controls remain).
   Extend the existing mailbox/replay state machine with user-visible queued,
   running, and cancelled control projections.
5. MCP lifecycle ownership — `harness-10`.
   Connect the tested transport/notification state machine to an owned runtime
   lifecycle without leaking tasks or sessions across actors.
6. Session and Git interactive UX — `harness-12` and `harness-13`.
   Complete picker/history/conflict interactions over the existing actor-owned
   projections and inverse-safe event boundaries.
7. IDE and noninteractive live boundaries — `harness-17` and `harness-18`; telemetry is now wired as one actor-owned live projection into provider streams and `/usage`.
   Add owned socket/editor and terminal metadata adapters over the typed event
   protocols, with deterministic abort/error replay.
8. Diagnostics and media completeness — `harness-16` and `harness-19`.
   Add remaining provider media formats and interactive diagnostic controls
   only after their renderer-neutral data contracts are covered.
