# p20 — Thin actor DSLs and YAML-first tests

**Aborted-turn fixture (2026-08-06):** `visual-aborted-turn.yaml` now keeps
the Pi-compatible partial-response abort contract editable as an event
sequence and state assertion without recompiling a scenario-specific test.

**Objective:** keep recurring actor ownership boilerplate small and make
behavior fixtures editable without recompiling Rust.

## Rules

- Macros remain thin and readable in `cargo expand`; they may remove repeated
  wiring but must not hide mailbox commands, state transitions, or event order.
- YAML is the default format for replay and TUI behavior scenarios. Rust tests
  provide the runner and only cover concurrency, timing, macro expansion, and
  serialization contracts that YAML cannot express.
- Every YAML fixture is self-describing and discovered by a test; orphaned or
  malformed fixtures fail loudly.

## Progress

- **YAML-owned tool schemas (2026-08-06):** `ToolSpec.parameters` now accepts
  a JSON Schema directly in a fixture. The new
  `visual-tool-schema.yaml` scenario sends string/number values through the
  real loop and tool actor, then asserts the coerced tool-result transcript
  and complete lifecycle event vector. Schema behavior can therefore be
  iterated without recompiling a fixture-specific Rust tool.

- **Fixture-owned terminal usage (2026-08-06):** `Done` event YAML may now
  declare `Usage` fields, which flow through the real assistant terminal event
  into the actor-owned status/footer projection. Shared `Usage` serde defaults
  keep compact fixtures concise while preserving the complete pi wire shape;
  `visual-hey.yaml` exercises the path and the full workspace gate remains
  green.

- **Exact functional TUI vectors (2026-08-06):** `hello-streaming.yaml` and
  `tool-echo.yaml` now assert complete ordered event vectors and terminal state,
  extending the event-sequence→state contract beyond the visual `Hey` case.

- **Typed tool-block state assertions (2026-08-06):** YAML `state` now accepts
  `tool_blocks` and `tool_output_lines`, backed by the actor snapshot's pure
  `Scrollback::tool_blocks()` projection. Structured, error, and execute tool
  fixtures exercise block/member counts without compiled fixture-specific code.
  Grouped activity fixtures also assert the preserved two-member block count
  while their visual assertions verify collapsed/selected visibility.
  `tool_modes` additionally pins ordered expanded/collapsed mode events.
`visual-activity-truncated.yaml` now pins the third `truncated` variant.
  Tool-card fixtures also pin ordered semantic headers and per-block output
  rows (`tool_headers`/`tool_outputs`), keeping the typed Grok block payload
  contract declarative.

- **TUI actor ownership reuse (2026-08-06):** `UiActor` and `PromptActor`
  now use the shared `runie-core` owned-worker DSL. Their worker lifetimes
  remain attached to cloned actor handles, while the duplicate local
  `JoinHandle`/`Drop` owner wrapper was removed. The actor ownership macro tests
  and full workspace gate verify the migration.

- **TUI mailbox DSL reuse (2026-08-06):** Both actors now also use
  `spawn_actor_worker!` for channel construction, so mailbox capacity and task
  ownership are established by the same thin DSL used by core actors. Command
  types and worker reducers remain explicit at each call site.

- **Subscriber ownership DSL reuse (2026-08-06):** The core loop's event
  subscriber bridge now uses `spawn_owned_worker!` instead of manually wrapping
  its `tokio::spawn` in `TaskOwner`. The loop run handle remains explicit
  because it carries a result and is synchronously awaited by the busy/idle
  coordinator.

- Added `spawn_owned_worker!`, a small internal DSL that constructs an
  aborting `TaskOwner` around an actor worker handle. Mailboxes and worker
  loops remain explicit at each call site.
- Added `spawn_actor_worker!`, a thin mailbox DSL shared by state, queue,
  provider, and tool actors; it centralizes channel creation plus owned worker
  attachment without hiding command enums or event handlers.
- Added `mailbox_call!`, a thin one-shot reply DSL used by both queue actors;
  it centralizes reply-channel creation and closed-mailbox defaults while
  leaving command constructors and actor-owned mutations explicit.
- Added direct mailbox-to-worker expansion coverage for `spawn_actor_worker!`;
  the macro test sends a command through the generated channel and verifies
  the owned worker lifecycle.
- Added `agent_event_kind!`, a readable declarative match shared by replay and
  integration harnesses so YAML event names have one source of truth.
- Added `assistant_event_kind!`, removing the handwritten provider-event
  naming match from the replay harness and covering the macro expansion.
- Extended `typed_action_registry!` with generated filtering, selection, and
  count helpers. The command palette now has one declarative action registry;
  its Pi-limited three-action scope remains explicit and YAML-editable.
- Existing `.sse.yaml` replay fixtures and TUI `tests/e2e/*.yaml` runners are
  the canonical fast behavior surface; the replay harness discovers all 183
  sidecars dynamically, and the TUI integration suite exercises all 22 YAML
  fixtures dynamically.
- Added the optional `core.ordered_events` YAML oracle and exercised it in the
  simple OpenAI trace; scenario-specific ordering is now editable without
  recompiling Rust.
- Added declarative `Shift+Enter`/`Alt+Enter` prompt steps and a multiline TUI
  fixture, keeping this behavior in the YAML scenario layer.
- Added a declarative `Ctrl+L` step and file-search prompt fixture.
- Added declarative `Shift+Tab` support and `visual-plan.yaml` coverage for the
  plan-mode prompt variant.
- Removed a timing-sensitive footer assertion from the typed visual fixture;
  it now asserts only the stable prompt content.
- Added `visual-tool-structured.yaml` coverage for multiline structured tool
  updates through the real tool executor and event renderer.
- Added a test-level YAML fixture discovery guard so malformed or orphaned TUI
  fixtures fail during `cargo test`.

## Completion

- Fixed declarative multi-tool replay indexing: each YAML `tool_call` now
  receives its event-sequence index instead of every call using index zero,
  preventing tool-call reconstruction from overwriting earlier calls. The
  mixed-activity fixture and full local CI pass.

All current parity additions have YAML scenarios before or alongside compiled
tests, and both thin DSL macros have expansion coverage. The rich markdown YAML
scenario now exercises blockquote gutters, six-level ATX headings, and table
box-drawing rows through the real TUI event path. Replay fixtures remain
runtime-discovered and editable without recompiling the runner.

**Canonical feed reducer (2026-08-06):** all existing TUI YAML scenarios now
enter `Scrollback` through the actor-owned `FeedState` reducer, including
tool-update/end identity and viewport events. The `visual-tool-update.yaml`
fixture's ordered lifecycle, header identity, and inactive-row assertions
therefore validate the production model boundary rather than a widget-local
compatibility reducer.
- **State assertion DSL (2026-08-05):** Added `assert_declared_state!` to
  the replay harness. YAML owns projection values while startup-error,
  abort, and success branches reuse one readable macro expansion for
  `is_streaming`, pending-tool, and error assertions.

- **Submitted-turn state coverage (2026-08-05):** Added declarative state
  assertions to `visual-submitted.yaml`, pinning the post-event state
  (`is_streaming: false`, no pending tools, two messages) alongside its visual
  transcript assertions.
- **Structured-tool state coverage (2026-08-05):** The structured-update
  fixture now also asserts the terminal core state declaratively, keeping tool
  rendering and functional message-count verification in one YAML scenario.

- **Typed registry DSL (2026-08-06):** Added the readable
  `typed_action_registry!` macro in `runie-core` and used it for TUI
  command-palette actions. The expansion generates the enum and label match;
  macro and palette registry tests cover known and unknown labels.

- **Registry SSOT (2026-08-06):** The macro-generated palette labels are now
  consumed directly by the pure command-palette widget; display filtering and
  actor activation no longer maintain separate label lists.
YAML visual steps now support `Up`/`Down` tool selection through the live app
actor; `visual-activity-mixed.yaml` asserts the resulting visible `⌄` marker for an expanded tool.

- **Acknowledged mailbox DSL (2026-08-06):** Added `mailbox_ack!`, a thin
  macro that keeps the command constructor visible while standardizing the
  one-shot acknowledgement protocol. Status and scrollback actors use it for
  live bus projections and replay commands; a macro test proves the caller
  cannot observe completion before the worker acknowledges reduction.
## Macro/DSL audit (2026-08-06)

- **Core-event UI mapping (2026-08-06):** `ui_messages_for_event` is now the
  pure SSOT mapping from `AgentEvent` to UI reducer messages. `UiActor` folds
  that mapping through `UiState::update` instead of embedding a special-case
  reset mutation in its worker. The event mapping has a model test; this keeps
  state delivery event-based while leaving the actor mailbox and acknowledgement
  protocol explicit.

- **Next YAML gap identified (2026-08-06):** The generic scenario outcome
  currently exposes core, feed, and status projections but not the actor-owned
  `UiState` projection. Therefore the new UI event mapping is covered by a
  model/actor test, not yet by a runtime-discovered YAML state assertion. The
  next DSL slice should add generic `state.ui` fields (welcome, shortcuts,
  palette visibility/query/index) and construct that projection through the
  same event sequence; no fixture-specific Rust test should be added.

- **UI projection DSL (2026-08-06):** Added a nested `visual.ui` YAML
  assertion block for welcome, shortcuts, and command-palette state. The
  palette New Session fixture now verifies the actor-owned reset projection
  after `Ctrl+P → new → PaletteEnter`, exercising the pure core-event mapping
  through the live mailbox without recompilation.

- **Shortcut overlay fixture (2026-08-06):** Added `visual-shortcuts.yaml`
  and a declarative `Ctrl+X` step. It validates the actor-owned shortcuts
  overlay and reset-safe palette fields through the runtime fixture discovery
  path.

- **Shortcut renderer extraction (2026-08-06):** Moved the pure shortcut
  overlay projection from the binary into `widgets::shortcuts`, then reused it
  from production and YAML rendering. This keeps declarative visibility in
  `UiState` and terminal painting in one renderer implementation.

- **UI assertion consolidation (2026-08-06):** Migrated all command-palette
  fixtures to `visual.ui` and removed the duplicated top-level palette
  assertion fields from the YAML runner. UI state now has one declarative
  schema and one comparison path.

- **Actor workflow state oracle (2026-08-06):** YAML `state.workflows` now
  compares the complete actor-owned workflow projection (identity, objective,
  phase/state, active-agent count, terminal status, and elapsed time) by stable
  run id. `visual-workflow-lifecycle.yaml` exercises the contract through the
  real event sequence, closing a previously visual-only workflow check without
  adding fixture-specific Rust code.

- **Thinking-level event/state oracle (2026-08-06):** YAML now supports the
  Pi `thinking_level_changed` control event and `state.thinking_level`, with
  one parser for all serialized levels. `visual-status-working.yaml` exercises
  the event-to-actor-state path without recompilation.

- **Feed projection state oracle (2026-08-06):** YAML `state` now also covers
  actor-owned feed flags for reasoning fold, activity fold, and live
  follow-latest behavior. Existing collapsed-activity and reasoning fixtures
  pin these reducer states alongside their rendered output.

- **Command-palette interaction oracle (2026-08-06):** Visual YAML steps now
  support palette Escape and can assert actor-owned `palette_open`, query, and
  selection index. `visual-command-palette.yaml` verifies Grok's first-Escape
  behavior: filtering is cleared while the modal remains open.

- **Context projection DSL (2026-08-06):** Scenarios can now declare
  `context.system_prompt` and prior text messages. The runner passes this
  through every prompt path, and `context-state.yaml` asserts the resulting
  actor-owned system prompt/message projection without recompilation.

The recurring actor ownership, event-kind classification, and TUI view-tree
patterns are already covered by thin local macros (`spawn_actor_worker!`,
`mailbox_*`, `agent_event_kind!`, `assistant_event_kind!`, and `view!`). A
further macro would hide payload semantics rather than reduce meaningful
boilerplate, so no new macro was introduced in this audit. The YAML DSL was
extended instead where it had a real pi parity gap: sectional text, thinking,
tool-call, and tool-update events are now data-driven and fixture-tested.
## Explicit viewport reducer events (2026-08-06)

The YAML DSL now supports `follow_latest: true|false`, mapped directly to the
actor-owned `ScrollbackMsg::SetFollowLatestUser` reducer input. Visual replay
applies this event after transcript reduction, so tool-update fixtures can
declare their viewport phase without sleeps or scheduler yields. The
`visual-tool-update` scenario uses it and asserts `follow_latest_user: true`.
## Assertion macro boundary (2026-08-06)

## Running-card fold replay (2026-08-06)

`tool_seed.running: true` is a test-only declarative lifecycle fact. It lets
YAML hold an arbitrary Grok card in the running state and apply `tool_fold`
before settlement, so the running-only `Collapsed -> Truncated` transition is
covered without timers, sleeps, or fixture-specific Rust. The
`visual-tool-running-fold.yaml` scenario asserts the resulting generic card
mode and running projection.

The YAML runner now uses a small `assert_yaml_eq!` macro for scalar
projection checks. It only expands assertion/diagnostic boilerplate; fixtures
remain runtime-loaded YAML and reducers, actors, and rendering stay ordinary
event-driven code. This keeps the DSL concise without hiding state ownership
or introducing macro-generated mutable state.

## Closed Pi-event assertions (2026-08-06)

Assertions may now declare `pi_events` alongside compatibility `exact_events`.
The runner converts every emitted event through `PiAgentEvent::try_from` and
fails if an application/TUI event crosses the Pi fixture boundary. The
`visual-hey` fixture exercises the complete user/assistant lifecycle without
requiring Rust recompilation when the YAML sequence changes.
Pi event tags are read from the generated serde representation rather than a
second hand-maintained variant matcher, keeping the YAML oracle coupled to the
macro-generated wire contract.
The same closed-boundary assertion now also covers the text-only
`hello-streaming` and `tool-echo` fixtures, giving the basic replay corpus
Pi-compatible event-vector evidence without requiring Rust recompilation.
`visual-tool-schema` extends that evidence through a complete tool-use,
continuation-turn, and terminal `agent_end` sequence while retaining its
state and transcript assertions.

Pure projection reuse (2026-08-06): the memory-search YAML path no longer
depends on renderer-local formatting. `memory_display_lines` is a model-owned
projection consumed by both live and replay event routes, keeping fixture
changes recompilation-free and eliminating a second formatting contract.

### Actor-owned context metrics (2026-08-06)

Zero-window normalization (2026-08-06): YAML `context_window: 0` now follows
the same event-boundary contract as the live model refresh: it delivers
`None` to `StatusActor`, preserving Grok's `500K` fallback. The new
`visual-context-window-default.yaml` fixture asserts both the actor state and
the full header meter without compiled scenario code.

The replay DSL now accepts `context_window: N` as a declarative event. It is
delivered to `StatusActor` as `StatusMsg::SetContextWindow`, rather than being
written into a renderer or inferred from a snapshot. A state assertion such as
`context_window: 1000000` validates the resulting actor snapshot, and the
`visual-context-window` fixture also exercises the screen projection. This
keeps context-meter parity event-driven and lets metric variants change in
YAML without recompiling the test harness.

The workflow terminal-state fixture also covers a cancelled phase state and
asserts Grok's fallback `○` glyph alongside the actor-owned workflow snapshot.

### State mailbox DSL reuse (2026-08-06)

`AgentStateActor` public mutators now route their explicit `StateCommand`
constructors through a small private acknowledgement helper backed by
`mailbox_ack!`. The macro removes repeated oneshot plumbing without hiding
payload semantics or changing the YAML fixture boundary.

The event-boundary regression is covered by the live actor construction path:
theme changes enter through `EventBus` and are reduced by each projection
actor, keeping the YAML/event model applicable to all TUI state owners.
