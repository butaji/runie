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

## Ranked extension queue after finite acceptance

The finite acceptance boundary in [`completion-matrix.md`](completion-matrix.md)
is implemented and verified for H01–H20. The queue below tracks only the
explicitly open “richer”, provider-specific, or platform-specific extensions;
these do not invalidate the finite matrix evidence.

## Reachable finish path

Each extension follows the same loop: add a failing event/replay test, add the
smallest actor-owned data boundary (use a declaration macro only for repeated
closed vocabulary), run `just ci`, run the 79-case TUI smoke, update the
finding, and push one focused commit.

Priority order:

1. Close provider/session transport gaps with captured wire fixtures and no
   live secret dependency. Session JSONL publication now uses collision-safe,
   uniquely staged temporary paths while retaining actor-owned atomic rename.
2. Add richer tool/scheduler/MCP controls only as typed commands and actor
   events, with queued/running/error replay cases.
3. Add session/Git picker and conflict interactions through existing pure
   projections; never mutate another actor directly.
4. Add remaining media formats only when a provider wire fixture and bounded
   MIME/data contract exist.
5. Finish diagnostics/TUI polish with renderer-neutral query state and
   deterministic fixture captures.

Completion evidence is: every finite matrix row has source + replay/unit test
 + live TUI evidence, the extension queue has no unverified claim, `just ci`
is green, the fresh smoke summary is `passed=79 failed=0`, and `git status`
 is clean after push.

The reduction backlog is now backed by declarative tables for the major closed
vocabularies, so the next work is functional parity. Each item remains open
until its source change, event/replay tests, and live TUI evidence are recorded.

1. Provider-specific request adapters — `harness-04` and `harness-14`.
   MiniMax effort projection is now covered by a pure request-body regression;
   model switching now resets to a model-declared default before opening the
   picker; the shared model effort accessor is exhaustively tested across all seven
   declared levels and preserves unsupported-level omission;
   replay usage mapping now preserves Anthropic cache-read/cache-write fields;
   finish-reason mapping now preserves raw chat values with a
   compatibility-tested tool-call path and unknown values fail closed as
   explicit errors; a model-derived provider request profile now centralizes
   OpenAI, Anthropic, Gemini, MiniMax, and generic effort-field selection;
   the shared HTTP request boundary and live MiniMax request body consume that
   profile without overwriting explicit payload fields, including Anthropic's
   nested `output_config.effort`, whose wire identity is
   now serializable and reverse-parsable for replay;
   typed failures now preserve bounded server retry guidance alongside status and
   retryability; complete the finite provider payload/finish-reason conformance matrix,
   including unsupported-effort behavior and normalized failures.
2. Tool output and background lifecycle UX — `harness-01` and `harness-03`.
   `/jobs cancel all` now reduces every running job through the actor mailbox;
   serializable `ToolCardSummary` and `BackgroundJobSummary` now expose card
   status plus shared bounded output facts and Unicode-safe previews; `/jobs`
   now includes the preview in its pure terminal row; explicit job-output
   reads now return actor-owned typed capture facts and terminal rows instead
   of a renderer-unaware string;
   ordered aggregate tool-card previews and core-owned background summary
   terminal rows now remove duplicate renderer formatting; add richer
   renderer-neutral output cards and owned lifecycle controls, with
   failure/cancellation replay traces; `BackgroundOutput` now carries the
   actor-owned command/status/exit context as a complete output card; palette-discoverable `Job Output`
   now exposes the actor-owned bounded capture directly; summary and output
   reads now share one pure output-metadata projection for facts, previews,
   and truncation; background job status queries now consume the domain
   status wire vocabulary instead of a duplicate command-local table.
3. Model-aware context policy — `harness-05` (live usage boundary implemented).
   Recovery and `/context` now use the active model’s declared window, and
   `/context` consumes a serializable `ContextReport` projection for its
   typed threshold decision, policy-input rows, and a checked-in YAML context
   fixtures for required, unknown-window, and explicitly disabled policy
   states; `/context compact [instructions]` routes manual recovery through
   the actor pipeline; remaining work is richer compaction controls.
   unknown zero-sized model windows now disable recovery rather than creating
   an unconditional compaction request.
4. Scheduler cancellation controls — `harness-08` (actor-owned metrics and `/jobs` projection implemented; status filters now expose running, completed, failed, and cancelled rows; palette actions now expose Cancel All Jobs and Clear Finished Jobs).
   A serializable scheduler metric-row projection now drives terminal lines,
   including separate queued/running cancellation counters;
   cancellation reason wire names are macro-generated in both directions for
   stable replay and compact serialization;
   replay events can now explicitly cancel queued versus running slots while
   retaining legacy cancellation compatibility;
   `/jobs scheduler` and palette-discoverable `Active Jobs` expose actor-owned
   scheduler projections; extend the existing mailbox/replay state machine
   with richer cancellation controls and live queued/running transitions.
5. MCP lifecycle ownership — `harness-10`; unified MCP status rows now own their stable terminal projection, and reconnect retry/exhaustion decisions are serializable data, so `/mcps` does not rebuild transport/index/status formatting in the TUI.
   The stdio actor now publishes ready/busy/failed/closed lifecycle state,
   `/mcps` projects it through the loop-owned executor, and registry-backed
   stdio calls reuse an actor-owned persistent session, and both stdio and HTTP
   actors expose ready/busy/failed/closed lifecycle projections; persistent HTTP
   registry-backed stdio/HTTP sessions now reuse actor-owned transports, and
   `/mcps` projects unified stdio/HTTP lifecycle rows with transport filters;
   lifecycle names use the shared lowercase wire vocabulary; palette actions
   now expose ready and failed status filters, and `/mcps` validation consumes
   the macro-generated lifecycle reverse parser and transport vocabulary.
6. Session and Git interactive UX — `harness-12` and `harness-13`; history rows now explicitly expose whether an undo target is available.
   `/git status`, `/git diff`, and `/git review` are now palette-discoverable and project the
   actor-owned bounded results through the same command-result dialog as conflicts.
   `/sessions` now preserves each row’s source path, `/sessions history` is
   palette-discoverable, `/sessions history <entry-id>` selects a history row
   through the session actor, and `/sessions pick [text]` opens the actor-owned
   resume picker; `/undo [count]` now repeats validated actor transitions; complete picker/undo history (including the palette-discoverable `/undo` action) and Git conflict interactions over the existing
   actor-owned projections and inverse-safe event boundaries.
7. IDE and noninteractive live boundaries — `harness-17` and `harness-18`; telemetry is now wired as one actor-owned live projection into provider streams and `/usage`; IDE snapshots now expose bounded serializable diagnostic rows with severity, location, and message terminal projections; JSONL metadata now preserves model name, model default effort, and effective compaction settings.
   JSONL now emits selected provider/model/context metadata; add owned
   socket/editor and terminal metadata adapters over the typed event
   protocols, with deterministic abort/error replay.
8. Diagnostics and media completeness — `harness-16` and `harness-19`; diagnostic report summaries now carry validated typed inspect/fix action data alongside their stable rows, Anthropic image URLs now use the provider’s remote URL source shape, Pi now preserves all shared URL-media variants, and user-question dialogs now project optional header/body metadata as data.
   A serializable diagnostic report-row projection now drives `/doctor`; add
   remaining provider media formats and interactive diagnostic controls only
   after their renderer-neutral data contracts are covered.
