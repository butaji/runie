# p18 — Architecture: actors-ssot audit, event ownership, no orphan spawns, linter rules

**Parity target:** clean architecture invariant (project AGENTS.md) enforced across all p01-p17 changes.

## Reference

- pi event contract this architecture must preserve: `agent-loop.ts:95-274` (event ordering), `proxy.ts` (streaming), `agent.ts` (state ownership) — see `~/Code/agents/pi/packages/agent/src/`.
- Project `AGENTS.md` (repo root): events-based single-source-of-truth actors; each state slice owned by exactly one actor; the only change mechanism is events published by the owner; handlers/tools/tests never mutate another actor's state directly; read-only projections rebuilt from events; every spawned task has an owner (`JoinHandle`/`JoinSet`/completion event), no orphan `tokio::spawn`.
- `lint-check/src/main.rs` enforces across `crates/runie-core/src/**.rs`: magic numbers `>= 1000` (named constants; exempt <1000, underscore, hex, HTTP/JSON-RPC codes, test code) and orphan `tokio::spawn`. Keep files ~400 lines, functions ~60 lines.

## Adapt to runie

1. Re-audit every new actor introduced by p05/p06/p07 (loop continuation, busy guard, turn hooks) and p12 (state projections) for:
   - single owner per state slice;
   - events as the only mutation mechanism;
   - projections rebuilt from events, not stored state.
2. Confirm every `tokio::spawn` in the new code stores its handle (JoinSet/JoinHandle) or is joined; no orphans (the `ProviderActor`, `ToolExecutorActor`, `QueueActor`s, `LoopActor`, recorder tasks already follow this — keep it).
3. Run `cargo run -p lint-check` — must be clean after all changes.
4. Keep `types.rs`/`driver.rs`/`executor.rs` under the ~400-line target; split files if they exceed it (e.g. split `driver.rs` if auto-continue/hooks grow it).
5. Update `AGENTS.md` if any new actor/convention (e.g. turn hooks, state projection rules) needs documenting.

## State machine / variants

Audit checklist per module (pass/fail):
- `loop/driver.rs` — loop owns run state; continuation via events; no direct cross-actor mutation.
- `state/actor.rs` — owns the state slice; projections pure.
- `queues/*` — each queue actor owns its queue; drained via commands.
- `provider/*`, `tools/*` — mailbox actors; spawned tasks owned/joined.
- `events/*` — single bus; subscribers read-only.

## Acceptance

**EventRenderer boundary audit (2026-08-06):** `EventRenderer::with_actors`
is the production constructor and attaches `ScrollbackActor` and `StatusActor`
as the only projection owners. The `Arc<Mutex<…>>` compatibility constructors
and `Projection::Legacy` variants are compiled only under `cfg(test)`; they
cannot create a second production source of truth.

- `cargo run -p lint-check` clean.
- `cargo clippy --workspace --all-targets` clean (fix pre-existing warnings this sweep touches).
- `cargo fmt --all -- --check` clean.
- `cargo check --workspace --all-targets` green.

## Progress

- **Tool update side-buffer boundary (2026-08-08):** `ToolExecContext` now
  carries the mutex-backed update collector only when no event bus is
  supplied. Live actor execution sets it to `None` and publishes every update
  through `EventBus`; direct deterministic executor tests retain the optional
  compatibility collector. This removes production mutable shadow state
  without changing the event sequence or replay behavior.

- **Replay provider counter (2026-08-08):** Replaced the replay adapter's
  mutex-protected scalar turn counter with an atomic counter. Reset and
  first/subsequent stream selection retain the same semantics without an
  async blocking lock or mutable collection state.

- **Subscriber registry ownership (2026-08-08):** Replaced the registry's
  `Arc<Mutex<RegistryInner>>` with an owned mailbox worker. Registration,
  unregistration, ordered application/Pi dispatch, and size queries now cross
  one actor command boundary; the worker owns subscriber mutation and awaits
  each listener in Pi registration order. The registry remains cloneable while
  its `TaskOwner` cancels the worker with the final handle.

- **Loop run ownership (2026-08-08):** Removed the redundant
  `Arc<Mutex<Option<JoinHandle<RunLoopOutcome>>>>` from `LoopActor`. The
  actor's single-run semaphore is now the sole admission/idle projection;
  `wait_for_idle()` awaits that permit, and the loop awaits its run directly.
  Focused loop-entry, TUI submission, and full workspace parity suites remain
  green.

- **Audited (2026-08-05):** `cargo run -p lint-check`, formatting, and
  workspace check pass. Strict `cargo clippy --workspace --all-targets
  -- -D warnings` was historically noisy during the initial audit; the current
  workspace gate is clean and remains required for every change.
- **Progress (2026-08-05):** Removed the low-risk unused imports/dead provider
  field, replaced manual defaults, corrected iterator usage, and removed two
  production unused-variable warnings. The strict clippy failure count is now
  dominated by structural complexity/large-function findings and test-only
  style findings.
- **Spawn audit (2026-08-05):** Reviewed all production `tokio::spawn` sites
  in core and TUI. Actor workers are owned by their mailbox actors; the loop
  stores its run handle; provider pumps are owned by the provider worker; and
  YAML recorder tasks retain and join their handles. No unowned production
  spawn was found. Strict clippy remains the outstanding architecture gate.
- **Clippy progress (2026-08-05):** Removed the unused test helper, converted
  a single-pattern match, and fixed test fixture field reassignments. The
  strict gate now reports 16 library findings (down from the earlier 26),
  primarily structural complexity, large actor command enums, and oversized
  functions that still need refactoring.
- **Structural progress (2026-08-05):** Introduced a named `TransformContext`
  callback type shared by `LoopDeps` and `RunLoopDeps`, removing both repeated
  type-complexity findings. Core workspace checks remain green; 14 library
  clippy findings remain.
- **Mailbox progress (2026-08-05):** Boxed `AgentMessage` payloads in the
  steering and follow-up actor commands. Queue behavior and tests remain green;
  both large-enum findings for those actors are resolved.
- **Provider mailbox progress (2026-08-05):** Boxed the model, context, and
  options carried by `ProviderCommand::Start`; provider replay and core tests
  pass, and the remaining clippy library findings are now 11 structural
  complexity/size issues.
- **Loop cleanup (2026-08-05):** Removed the redundant `into_iter()` in the
  steering/follow-up continuation chain. Event-sequence and core unit tests
  remain green; 10 structural clippy findings remain.
- **Parser decomposition (2026-08-05):** Split streaming-JSON salvage into
  focused close/finalize helpers; parser tests pass and strict core clippy is
  now down to 8 findings. Remaining issues are the large loop/replay/tool
  functions plus their cognitive-complexity thresholds.
- **Dispatch/parser decomposition continued (2026-08-05):** Extracted replay
  line classification and tool-result/event construction into pure helpers.
  Core unit tests pass and the cognitive-complexity findings for replay and
  parallel dispatch are resolved. Remaining strict core clippy findings are
  function-size thresholds in the loop driver, replay parser, tool dispatcher,
  and truncated-call helper; the loop driver remains the main architectural
  split still required.
- **Atomic streaming projection (2026-08-05):** Added a state-owner command
  that updates `is_streaming` and `streaming_message` together. The loop now
  publishes one coherent projection at stream start/end, eliminating an
  observable intermediate state; the full workspace test suite and all 22 TUI
  visual tests pass.
- **Driver stream-phase split (2026-08-05):** Extracted provider startup,
  assistant streaming, event publication, and atomic projection updates into
  `stream_assistant`. Core unit tests remain green; the main driver fell from
  272 to 224 lines and cognitive complexity from 60 to 48. Remaining clippy
  work is further decomposition of the turn/tool phase plus small replay/tool
  size thresholds.
- **Stream drain split (2026-08-05):** Moved broadcast event draining into
  `drain_assistant_events`; the extracted stream helper now satisfies the
  local size/complexity checks. Core unit tests remain 32/32 green. Remaining
  findings are the coordinator turn/tool body and three small size-only
  helpers.
- **Batch preflight/finalization split (2026-08-05):** Extracted parallel
  tool preflight and replay event finalization. Core unit tests remain green;
  strict core clippy is now limited to the main loop coordinator and two
  near-threshold dispatch/parser functions.
- **Parallel completion split (2026-08-05):** Extracted completion-order
  execution and collection into `run_parallel_calls`. Tool dispatch now
  satisfies strict core clippy; only the loop coordinator and one replay
  parser helper remain over the local size thresholds.
- **Replay provider split (2026-08-05):** Separated Anthropic and OpenAI tool
  call collection paths. All replay fixtures pass, `lint-check` and format
  checks pass, and strict core clippy now reports only the main `run_loop`
  coordinator.
- **Queue injection split (2026-08-05):** Extracted steering/follow-up
  draining and event-backed message injection from `run_loop`. Core unit
  tests remain green; the coordinator is down to 200 lines and cognitive
  complexity 39. Remaining work is the turn/tool branch decomposition.
- **Tool-batch orchestration split (2026-08-05):** Extracted pending-call
  ownership, tool execution, event publication, result-message projection, and
  turn-end emission into `run_tool_batch`. Core unit tests remain 32/32 green;
  the coordinator is now 147 lines with cognitive complexity 28. The helper
  itself needs one final publication/execution split before the strict clippy
  architecture gate is clean.
- **Tool execution split (2026-08-05):** Moved provider/tool actor dispatch
  and aborted-run handling into `execute_tool_calls`; all core unit tests
  remain green and the helper is now below size limits. Strict clippy has only
  the loop coordinator and its compact batch publication helper remaining.
- **Tool publication split (2026-08-05):** Extracted event/result projection
  and `TurnEnd` publication into `publish_tool_outcome`. Core unit, replay,
  workspace, lint, and format checks pass; strict core clippy now reports only
  the main `run_loop` coordinator (28 complexity, 147 lines).
- **Architecture gate closed (2026-08-05):** `just ci` now passes completely:
  formatting, strict workspace clippy, lint-check, and the full workspace test
  suite. The actor spawn audit remains clean, including owned TUI renderer and
  provider pump tasks; this step is complete.
- **Lifecycle ownership refinement (2026-08-05):** `EventRenderer` now tracks
  the `TurnStart` boundary explicitly and emits completion duration only for a
  real started turn. Synthetic/session setup `AgentEnd` events no longer
  mutate the transcript with a false `Worked for 0.0s` projection; the reducer
  regression pins both paths.
- **TUI shutdown ownership (2026-08-05):** The terminal binary now signals
  and awaits its renderer `JoinHandle` on quit instead of dropping the handle
  in a discarded tuple, closing the last identified production-task lifetime
  gap.
- **Projection acknowledgments (2026-08-05):** Pending-tool state commands
  now carry an actor acknowledgment, so callers return only after the owning
  state actor has applied the projection. The projection test waits on the
  tool's explicit start boundary rather than polling scheduler progress.
- **Turn-hook split (2026-08-05):** Extracted stop-after-turn evaluation and
  prepare-next-turn model/context/thinking overrides into `apply_turn_hooks`.
  Core unit tests remain 32/32 green; `run_loop` is now 132 lines with
  cognitive complexity 22. The remaining strict clippy work is the outer
  turn coordinator split itself.
- **Outer-turn split (2026-08-05):** Extracted initialization, assistant-turn
  orchestration, assistant plan handling, abort checks, and run finalization.
  Strict `cargo clippy -p runie-core --lib -- -D warnings` now passes, with
  all 32 core unit tests green. This resolves the core library architecture
  lint gate; workspace-wide test/lint verification remains the final audit.
- **Final local verification (2026-08-05):** `cargo test --workspace --quiet`,
  replay tests, `cargo run -p lint-check`, and `cargo fmt --all -- --check`
  all pass. The core library strict clippy gate also passes; remaining audit
  work is workspace-wide strict clippy/test-only warning cleanup and broader
  exact parity review.
- **Workspace clippy inventory (2026-08-05):** Refactored `lint-check` into
  focused project/file/line scanners; its strict clippy findings are gone.
  Workspace-wide strict clippy now reaches TUI and test targets and reports
  remaining renderer/YAML-runner size/complexity functions plus test-harness
  dead-code/type-complexity warnings. Converted TUI `Status` to a derived
  default as the first low-risk cleanup; the full workspace clippy gate stays
  open pending those decompositions.
- **Scrollback layout split (2026-08-05):** Extracted physical-row construction
  and width wrapping from `Scrollback::render` into pure helpers. All 14
  scrollback tests pass and its render function no longer appears in strict
  clippy findings; markdown styling and Grok transcript geometry are unchanged.
- **Status footer split (2026-08-05):** Separated buffer placement from pure
  footer-line construction in `StatusBar`. All status tests pass and the
  public render method is below the size threshold; remaining work is the
  context-sensitive footer helper plus larger event/prompt/YAML renderers.
- **Status branch split (2026-08-05):** Split ready/loading/active/fallback
  footer spans into pure builders. All status tests remain green and the
  status widget no longer appears in strict clippy findings.
- **Event lifecycle/message split (2026-08-05):** Extracted `AgentStart`
  reset/welcome handling and `MessageStart` projection handling from
  `EventRenderer::apply_event`. Event-renderer tests remain green; the
  reducer fell from 175 to 149 lines and cognitive complexity 21→20.
- **Welcome layout split (2026-08-05):** Separated compact and full-mode
  welcome rendering into dedicated methods. All three welcome tests pass and
  the welcome widget no longer appears in strict clippy findings; Grok
  geometry/snapshots remain unchanged.
- **Message-update split (2026-08-05):** Collapsed all assistant
  `MessageUpdate` variants into `handle_message_update`, preserving text,
  reasoning, completion, error, and tool-call behavior. Event-renderer tests
  remain 7/7 green; `apply_event` fell from 149 to 106 lines and cognitive
  complexity 20→18.
- **Tool lifecycle split (2026-08-05):** Extracted tool start/update/end and
  message-end handling into focused event handlers. All 7 event-renderer tests
  pass; `EventRenderer::apply_event` now satisfies strict clippy, removing the
  largest remaining TUI reducer finding.
- **Prompt render split (2026-08-05):** Separated prompt border drawing,
  caption construction, and input-line construction into focused helpers. All
  14 prompt tests pass; prompt rendering no longer appears in strict clippy
  findings and its exact geometry remains covered by snapshots/tests.
- **Transcript markdown/key cleanup (2026-08-05):** Split inline markdown
  parsing from block/bold parsing, extracted assistant-line styling, and
  simplified modifier-first key routing. Scrollback and key tests pass; strict
  TUI clippy is now limited to the YAML scenario runner functions.
- **YAML scenario orchestration split (2026-08-05):** Extracted scenario loop
  construction, recorder execution, synchronous transcript replay, and prompt
  submission. TUI tests remain green; `run_scenario` now satisfies strict
  clippy. Remaining YAML findings are assertion and visual-buffer helpers.
- **TUI gate inventory (2026-08-05):** Marked shared integration-test fixture
  dead code/type aliases explicitly and extracted the Ctrl+C key policy into a
  pure helper. Workspace tests remain green. Remaining production findings are
  concentrated in `EventRenderer`, prompt/scrollback rendering, welcome and
  status widgets, and YAML scenario rendering; these require behavior-aware
  MVU/event-handler splits rather than blanket lint suppression.
- **Prompt reducer split (2026-08-05):** Extracted submit, clear, history
  navigation, and character insertion handlers from `PromptWidget::handle_key`.
  All 62 TUI unit tests pass; the key reducer no longer appears in the strict
  clippy findings. Remaining prompt work is render decomposition alongside the
  event renderer and transcript widgets.
- **Workspace clippy inventory (2026-08-05):** Refactored `lint-check` into
  focused project/file/line scanners; its strict clippy findings are gone.
  Workspace-wide strict clippy now reaches TUI and test targets and reports
  remaining renderer/YAML-runner size/complexity functions plus test-harness
  dead-code/type-complexity warnings. Converted TUI `Status` to a derived
  default as the first low-risk cleanup; the full workspace clippy gate stays
  open pending those decompositions.
- YAML visual assertion/render split: extracted event, transcript, and visual assertion helpers, isolated the deterministic frame renderer, and corrected idle visual frames so turn status only appears during typed replay. TUI unit, visual snapshot, replay, formatting, lint, and strict core/TUI clippy gates pass.
- **Current gate inventory (2026-08-05):** strict workspace clippy still
  reports test-target line-length/style findings in shared fixtures and a
  handful of parity tests, plus the long status fixture test. These are
  verification cleanup items; production core/TUI library clippy remains
  clean.
- **Workspace gate closed (2026-08-05):** scoped the remaining fixture-only
  clippy findings with rationale, removed unused test imports/mutability, and
  reached a clean `cargo clippy --workspace --all-targets -- -D warnings`.
  Full workspace tests, replay tests, formatting, and `lint-check` also pass.
- **Explicit task ownership (2026-08-05):** added the internal `TaskOwner`
  lifetime guard to state, steering, follow-up, provider, and tool actors;
  provider stream pumps now live in the provider worker's `JoinSet`. The loop
  actor continues to retain and join its run handle. Core unit tests and
  strict core clippy pass after this ownership change.
- **TUI projection ownership (2026-08-05):** removed the App controller's
  direct `Status` transitions around prompt execution; status now follows the
  core event renderer's `AgentStart`/`TurnStart`/`MessageUpdate`/`AgentEnd`
  projection. Shared locks are the read-side synchronization boundary for
  terminal rendering; production mutation remains renderer-owned.
- **Reactive loop ownership (2026-08-05):** active provider streaming now
  reacts directly to the loop-owned abort watch; the abort transition and
  partial-message terminal state are covered by a deterministic integration
  test.
- **Provider cancellation ownership (2026-08-05):** provider cancellation now
  aborts its owned pump set instead of being a no-op; the pending-stream actor
  test verifies the receiver closes deterministically.
- **TUI animation ownership (2026-08-05):** moved status animation ticks out
  of the terminal controller and into the owned `EventRenderer` task. The
  binary now only renders the projection and handles input; status mutation is
  centralized in the renderer's event/tick loop.
- **TUI projection ownership audit (2026-08-05):** rechecked every
  `Scrollback`/`StatusBar` mutation. Production writes are confined to the
  owned `EventRenderer` task; the binary and renderer callers only read for
  drawing. The remaining YAML-runner `clear()` calls are deterministic
  scenario setup/reset operations, isolated to test infrastructure.
- **Event-to-projection boundary (2026-08-05):** added
  `AgentStateActor::apply_event` with focused message and tool event mapping,
  and routed the main input, assistant terminal, and tool-result paths through
  that boundary. The state actor remains the sole projection mutator. The
  earlier note that direct streaming/error transitions remained open is
  superseded: current driver paths publish `MessageStart`, `MessageUpdate`,
  `MessageEnd`, or `Error` through `publish_and_apply`, and the focused state
  projection tests cover those transitions.
- **Verification (2026-08-05):** `just ci` passes after the event-to-state
  routing change.
- **Terminal error projection (2026-08-05):** terminal assistant
  `error_message`/aborted metadata now flows through the published
  `MessageEnd` event into the state actor; abort streaming no longer mutates
  the error projection directly. Provider-startup and tool-abort error paths
  remain tracked for the same treatment.
- **Verification (2026-08-05):** `just ci` remains green after terminal error
  routing.
- **Projection regression (2026-08-05):** Added a deterministic actor test
  proving `MessageStart`/`MessageEnd` events alone establish streaming,
  terminal error, and transcript projections without direct field mutation.
- **Verification (2026-08-05):** focused projection test and full `just ci`
  both pass.
- **Executor-drop parity (2026-08-05):** Tool executor mailbox failure now
  synthesizes pi-compatible error tool results and
  `ToolExecutionStart`/`ToolExecutionEnd` events from the actor-owned call
  list, rather than mutating global agent error state. Added a focused
  regression test for the fallback contract.
- **Verification (2026-08-05):** `just ci` passes after the executor-drop
  fallback change.
- **Pending-tool event ownership (2026-08-05):** Removed the loop driver's
  direct pending-tool state mutations. Normal tool starts are now published
  and applied through `ToolExecutionStart` before tool side effects begin;
  completion and truncated-call paths use the same event projection boundary.
  The in-flight projection regression remains green.
- **Workspace strict-clippy verification (2026-08-05):** Re-ran
  `cargo clippy --workspace --all-targets -- -D warnings`; it completes cleanly
  with no warnings. The architecture lint gate is therefore green alongside
  `just ci` and `lint-check`.
- **Non-message error event ownership (2026-08-05):** Abort, tool-batch abort,
  and provider-without-stream paths now publish an `AgentEvent::Error`; the
  state actor alone applies that event to `error_message`, and the TUI derives
  its error status from the same event. Added projection coverage and retained
  the YAML event-kind mapping.
- **Verification (2026-08-05):** Core tests, workspace clippy, and the full
  workspace test suite pass after routing these error transitions through the
  event boundary.
- **Projection write sweep (2026-08-05):** Removed the remaining loop-side
  projection writes for queued-message insertion, provider startup cleanup,
  and hook-driven thinking-level changes. They now publish/apply
  `MessageEnd`, `Error`, and `ThinkingLevelChanged` events respectively.
- **Verification (2026-08-05):** `cargo fmt`, strict workspace clippy, and
  `cargo test --workspace` pass after the sweep.
- **Lifecycle projection ownership (2026-08-05):** Loop reset now publishes
  and applies a typed `Reset` event; the state actor and TUI clear their owned
  projections from that event. Event-kind DSL and YAML runner mappings cover
  `Reset` and `ThinkingLevelChanged`.
- **Verification (2026-08-05):** Strict workspace clippy and the complete
  workspace test suite pass after routing lifecycle reset through the bus.
- **Streaming projection boundary (2026-08-05):** Removed the driver's direct
  `set_streaming_state` mutation. Pi-visible assistant delta events are now
  cloned into `MessageUpdate`, published on the shared bus, and applied by the
  state actor; structural provider markers remain internal so exact pi event
  ordering is unchanged.
- **Verification (2026-08-05):** The six-case core event-sequence oracle and
  strict workspace clippy pass after this boundary change.
- **Publish/apply ordering (2026-08-05):** Tool-start and tool-result
  projections now publish before state application, matching the reactive
  bus contract. Consolidated the repeated sequence in a small
  `publish_and_apply` helper so new driver events cannot accidentally update
  the projection before subscribers receive them.
- **Verification (2026-08-05):** Core event-sequence tests and strict
  workspace clippy remain green.
- **Subscriber bridge (2026-08-05):** Wired `LoopActor::subscribe()` to the
  shared event bus with an owned async bridge task. Registered subscribers now
  receive every published event in registry order; the bridge is held by a
  `TaskOwner` and is aborted with the actor rather than detached.
- **Verification (2026-08-05):** Core library tests and strict workspace
  clippy pass with the bridge attached during actor construction.
- **Bridge regression (2026-08-05):** Added a deterministic async test that
  registers a subscriber, publishes `AgentStart`, and awaits delivery through
  the owned bridge; the core library suite now reports 50 passing tests.
- **TUI render purity (2026-08-05):** Removed model-caption mutation from
  `App::render`; caption refresh remains an explicit pre-draw/update action,
  leaving layout and widget drawing read-only.
- **Verification (2026-08-05):** TUI clippy and all 79 TUI unit, E2E, and 23
  visual tests pass after the render-purity change.
- **TUI MVU actor (2026-08-05):** Replaced ad-hoc `show_welcome` and
  `shortcuts_open` mutations with an async `UiActor` mailbox, pure `UiState`
  reducer, watch-based read-only snapshots, and acknowledged `UiMsg` updates.
  Terminal input and YAML scenarios now send the same messages; rendering
  only reads the snapshot.
- **Verification (2026-08-05):** Full `just ci` passes with 50 core tests,
  80 TUI tests, replay, E2E, visual parity, lint, and clippy.
- **Prompt MVU actor (2026-08-05):** Moved `PromptWidget` state behind an
  acknowledged async `PromptActor` mailbox. Key handling, clear, mode cycle,
  file search, and caption updates are actor commands; rendering and cursor
  geometry use read-only snapshots. YAML and terminal input share the actor
  path.
- **Prompt ownership verification (2026-08-05):** TUI clippy and all 80 TUI
  tests, including YAML E2E and visual parity checks, pass after the prompt
  actor migration.
- **Compile-time render purity (2026-08-05):** Changed `App::render` to take
  `&self`, making the draw path unable to mutate controller or actor state;
  actor snapshots and event-owned projections remain the only render inputs.
- **Verification (2026-08-05):** Strict workspace clippy and all TUI tests
  pass after tightening the render API.
- **Bus-reactive UI model (2026-08-05):** `UiActor` now consumes the shared
  core event bus alongside its input mailbox; lifecycle `Reset` events reset
  the UI model through the reducer. Added an async regression test without
  sleeps or polling delays.
- **Verification (2026-08-05):** The bus-reset actor test and strict workspace
  clippy pass.
- **Reactive lifecycle wiring (2026-08-05):** `UiActor` now subscribes to the
  core event bus and reduces `Reset` directly from lifecycle events, in
  addition to acknowledged input messages. The new test proves the bus event
  resets a previously hidden welcome state; no sleeps or polling delays are
  used.
- **Verification (2026-08-05):** Full `just ci` passes with 50 core tests,
  81 TUI tests, replay, E2E, visual parity, lint, and clippy.
- **Prompt lifecycle reactivity (2026-08-05):** `PromptActor` now also
  subscribes to the shared event bus and resets its buffer/history projection
  on `AgentEvent::Reset`. Added a no-sleep async regression test alongside the
  UI actor reset test.
- **Verification (2026-08-05):** Prompt/UI actor reset tests and strict
  workspace clippy pass.
- **Full TUI reset ownership (2026-08-05):** `PromptActor` now consumes the
  same bus `Reset` event as `UiActor`, clearing the prompt buffer/history
  projection through its own actor worker. Added deterministic async coverage
  for both actors.
- **Verification (2026-08-05):** Full `just ci` passes with 50 core tests,
  82 TUI tests, replay, E2E, visual parity, lint, and clippy.
- **SSOT write audit (2026-08-05):** Re-scanned core/TUI production paths for
  projection-field writes, actor commands, event publication, and spawned
  tasks. Projection mutations remain confined to `AgentStateActor::apply`; UI
  and loop code publish events or send actor commands, and existing spawn
  owners remain retained. YAML replay now asserts the resulting snapshots,
  providing a behavioral guard for this boundary.

- **State publication boundary (2026-08-06):** Replaced loop-level
  `bus.publish` plus direct `state.apply_event` pairs with the actor-owned
  `AgentStateActor::publish_event` boundary. The state actor now owns the
  coupled publication/projection operation, including prompt, assistant,
  tool-result, error, and reset events; a regression proves the bus event and
  actor snapshot are produced from one call without sleeps or polling.
- **Atomic event reduction (2026-08-06):** State event projection now uses a
  single acknowledged `ApplyEvent` mailbox command rather than several
  asynchronous setter commands. This makes the snapshot complete before the
  publication boundary returns and removes a scheduler-dependent visibility
  window for provider context construction.
- **Core workflow ownership (2026-08-06):** Workflow lifecycle events are no
  longer discarded by `AgentStateActor`. Its immutable snapshot now exposes a
  typed `workflows` map keyed by `run_id`, and the actor reducer owns
  start/progress/finish transitions. A direct lifecycle test covers the
  reducer; compatibility mutex paths for the legacy renderer remain an
  explicit migration target.
# Latest actor-boundary correction (2026-08-06)

The tool-batch completion `turn_end` path was the last live driver publication
that bypassed `AgentStateActor`. It now uses the shared `publish_and_apply`
boundary, keeping tool results, turn errors, and continuation decisions in the
same ordered event stream consumed by the actor projection.
