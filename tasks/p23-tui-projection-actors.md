# p23 — TUI projection actors

## Objective

Remove shared mutable `StatusBar` and `Scrollback` state from the TUI. Each
projection must have one actor owner, receive event-derived messages, and
publish immutable `watch` snapshots to the pure view.

## Current evidence

- `StatusMsg` and the pure `StatusBar::apply` reducer exist and are tested.
- `StatusActor` owns a status reducer worker, acknowledges commands, and
  publishes `StatusBar` snapshots through `watch`.
- `status_messages_for_event` is a pure, ordered mapping from core events to
  status reducer messages, with a regression test.
- `StatusActor::apply_event` now applies that mapping with command
  acknowledgments; actor-level event application has a regression test.
- Event batches are applied atomically and publish one snapshot, preventing
  observers from seeing `BeginTurn` without its corresponding phase state.
- The production `EventRenderer::run` now lazily owns and feeds a
  `StatusActor` from the event bus. Construction remains runtime-independent
  for synchronous reducer tests; all 106 TUI tests and visual fixtures stay
  green after this dual-projection step.
- `StatusActor::subscribe` exposes only a `watch::Receiver<StatusBar>`;
  regression coverage verifies one coherent publication for a batched
  `TurnStart` event and no intermediate snapshot.
- `ScrollbackMsg` and `Scrollback::apply` now define the explicit feed reducer
  surface, including append, row replacement, normalization, display modes,
  and theme changes; a reducer regression covers append/replace/clear.
- `ScrollbackActor` now owns that reducer behind an acknowledged batch
  mailbox and publishes read-only `watch` snapshots, with an actor-level feed
  regression test. Live renderer cutover remains pending.
- The production event loop now mirrors each completed feed reduction into
  `ScrollbackActor` as one atomic snapshot, and the live binary renders from
  that actor snapshot after startup. The legacy feed mutex remains only as a
  transitional reducer source and compatibility path.
- YAML visual rendering now reads status and scrollback through the same
  snapshot APIs, so its assertions exercise actor-owned views rather than
  directly locking projection state.
- `App::render` now also consumes the scrollback snapshot, removing its direct
  mutable feed lock from the general view path. Visual and YAML suites remain
  green.
- Animation demand and ticks now use only the actor-owned status snapshot in
  the async renderer, keeping spinner frames reactive without a production
  mutation of the compatibility status projection.
- Basic event status mapping is now actor-only in production; the synchronous
  compatibility reducer applies it only when no actor is attached. This
  removes duplicate writes for turn, waiting, theme, error, and completion
  events while retaining the legacy unit-test constructor.
- Assistant lifecycle status transitions (`TextDelta`, `ThinkingDelta`,
  `Done`, and assistant errors) are now included in the pure event mapping;
  production message handling no longer writes those status values to the
  compatibility mutex.
- Reset and agent-end status handling now reads/applies through the actor in
  production; elapsed completion text is projected from the actor snapshot.
- Reset, theme, and tool-display feed events now apply `ScrollbackMsg` batches
  directly to the actor in production; their compatibility mutations are
  disabled when an actor is attached.
- Added a pure feed-event mapping regression covering reset, theme, and the
  intentional no-op behavior for unrelated turn events.
- Event-bus lag recovery now appends its skipped-event system row directly to
  `ScrollbackActor`, so production recovery state no longer mutates the
  compatibility feed mutex.
- Fixed the dual-projection boundary so actor-owned reset/theme/tool-mode
  feed events are not overwritten by the legacy snapshot mirror.
- `App::status_snapshot` and the live binary now render from the actor-owned
  snapshot after renderer startup; the legacy mutex is used only for the
  initial frame and compatibility event-renderer tests.
- Existing `EventRenderer` and rendering paths still use compatibility mutexes.

- **Actor construction boundary (2026-08-06):** `EventRenderer::with_actors`
  now constructs its projections directly in actor mode. Production startup
  therefore does not allocate or retain a compatibility mutex for either
  scrollback or status; the private `Projection<T>` enum makes accidental
  legacy locking fail loudly. The remaining mutex-backed constructors are
  deliberately limited to synchronous reducer tests and compatibility replay,
  so this task stays open until those adapters are removed or isolated behind
  a separately named test-only surface.

- **Actor-only regression (2026-08-06):** Added a focused async renderer test
  that constructs `with_actors`, replays `AgentStart` and user
  `MessageStart`, and asserts both actor snapshots. This protects the
  production path against accidentally reintroducing legacy projection access
  while keeping the event-to-snapshot contract explicit.

- **Production actor boundary audit (2026-08-06):** Rechecked every remaining
  `Arc<Mutex<Scrollback/StatusBar>>` constructor and lock path. They are
  compiled only under `cfg(test)` as synchronous compatibility adapters;
  production `Projection<T>` contains only the actor variant, and
  `App::spawn_renderer` constructs `EventRenderer::with_actors`. No production
  TUI projection has a mutex-backed source of truth.

## Latest progress

- The production `e` activity-fold action now publishes
  `ScrollbackMsg::ToggleActivityExpanded` to `ScrollbackActor` and awaits its
  acknowledgement instead of mutating the legacy mutex projection after
  actor startup.
- Completed-turn summaries now reduce through `ScrollbackMsg::AppendTurnSummary`
  in the production actor path; the legacy handler only appends when no actor
  has been started. The actor test verifies the blank separator and summary
  are published as one acknowledged transition.
- Message-update and assistant-message errors now enter the actor mailbox as
  `ScrollbackMsg::Append` system rows; production handlers retain the legacy
  mutation only for pre-actor compatibility construction.

- **Message-start projection migration (2026-08-05):** User and assistant
  `MessageStart` events now map purely to acknowledged `ScrollbackMsg` batches
  before the compatibility renderer runs. This gives the actor the canonical
  initial user/assistant rows (including the transient thinking block) and
  preserves the YAML event-to-snapshot contract. Unit and full fixture replay
  gates remain green.

- **Session-start projection migration (2026-08-05):** `AgentStart` now
  publishes the standard session-start rows or the welcome-modal rows directly
  to `ScrollbackActor` before compatibility rendering. Both actor-owned startup
  variants are covered by the existing unit/YAML gates.

- **Assistant delta projection migration (2026-08-05):** Text and reasoning
  deltas now map to the explicit `AppendToLastByKind` reducer message. The
  actor owns creation of the first reasoning row and accumulation of all later
  chunks, with mapping/reducer tests and YAML replay coverage kept green.

- **Assistant finalization migration (2026-08-05):** `MessageEnd` now emits
  `FinalizeAssistant` to the feed actor. The reducer owns removal of the
  transient thinking row, collapsed `Thought` conversion, and expanded-mode
  reasoning retention; the compatibility renderer follows the same event for
  legacy tests. Unit and YAML replay gates remain green.

- **SSOT correction (2026-08-05):** Finalization now reads the expansion mode
  from the `ScrollbackActor` snapshot rather than the compatibility mutex,
  removing the last compatibility-state decision from that actor transition.
  Clippy and the repository lint remain clean.

- **YAML view cutover (2026-08-05):** The visual YAML runner now renders from
  `App::scrollback_snapshot()` instead of locking the compatibility scrollback
  mutex. YAML assertions therefore exercise the same actor-owned view used by
  the live app; unit, replay, Clippy, and lint gates remain green.

- **YAML setup cutover (2026-08-05):** Visual setup mutations (reasoning and
  activity expansion, deterministic timestamps, waiting cleanup, spacing
  normalization, and reset) now use `App::apply_scrollback*`, so they update
  the actor snapshot consumed by rendering. The legacy mutex is retained only
  when an app has no renderer actor yet.

- **Typed-frame reset cutover (2026-08-05):** Clearing synthetic welcome rows
  after YAML prompt editing now also goes through the feed actor, preventing
  the setup phase from mutating a snapshot that the renderer does not read.

- **Scenario replay projection cutover (2026-08-05):** `run_scenario` now
  returns the final feed through a `ScrollbackActor` snapshot rather than
  exposing the compatibility reducer's lines directly. The stateful
  compatibility renderer remains an input adapter during migration, while
  YAML assertions observe the same actor-owned projection surface as the live
  view.

- **Agent-start duplicate-write removal (2026-08-05):** The live renderer no
  longer appends startup/welcome rows to the compatibility scrollback after
  the actor has been attached; those rows come exclusively from the ordered
  `ScrollbackMsg` batch. The synchronous constructor retains the compatibility
  branch for reducer tests.

- **Message-start duplicate-write removal (2026-08-05):** User and assistant
  row writes are now actor-only after startup. User timestamps are projected
  as `ScrollbackMsg::SetPromptTimestamp` alongside the user row, preserving
  live timestamp parity without consulting the compatibility mutex.

- **Message-update duplicate-write removal (2026-08-05):** Text and thinking
  deltas now update only the actor-owned assistant/reasoning rows in production;
  the compatibility handler still accumulates stream buffers and retains its
  row reducer only for synchronous renderer tests.

- **Message-end duplicate-write removal (2026-08-05):** Assistant finalization
  now occurs only through `ScrollbackMsg::FinalizeAssistant` after actor
  startup. The compatibility path retains the collapsed/expanded fold reducer
  for synchronous tests, while production no longer mutates those rows twice.

## Migration sequence

### Progress — 2026-08-05

- Promoted the owned-worker DSL across the crate boundary and reused
  `spawn_actor_worker!` for TUI projection actors. The macro now centralizes
  channel/owner construction while mailbox reducers remain explicit; macro
  expansion remains readable and the existing ownership tests cover it.

- Added `EventRenderer::with_actors` and switched production
  `App::spawn_renderer` to construct the renderer with its actor handles
  already attached. This removes the public-field mutation step from the live
  startup path. The compatibility constructors remain only for YAML replay
  and focused synchronous reducer tests.

1. Make event application async at the renderer boundary and await status
   command acknowledgments.
2. Replace `Arc<Mutex<StatusBar>>` in `App`, `EventRenderer`, the live binary,
   and YAML runner with `StatusActor` plus read-only snapshots.
3. Introduce the analogous `ScrollbackActor` using a pure message reducer.
4. Remove the compatibility mutexes and add YAML event-sequence assertions
   for snapshot state and rendered output.

## Acceptance

- No production TUI `Arc<Mutex<StatusBar>>` or `Arc<Mutex<Scrollback>>`.
- View code only reads snapshots.
- Every projection command is event-derived and acknowledged before a test
  asserts its resulting state.
- Existing visual, replay, and four-geometry cast checks remain green.

## Completion status

Complete for the production architecture boundary. Test-only compatibility
constructors remain intentionally available for pure reducer/unit replay; they
are not part of the shipped actor graph or production state ownership.

### Scrollback mutation inventory

The feed currently has distinct mutation families: append/clear, activity and
tool-row replacement, theme/tool-mode changes, reasoning/activity expansion,
timestamp and spacing normalization, and render-time live normalization. These
must become explicit reducer messages; a generic closure command is forbidden
because it would conceal ownership and make YAML event assertions impossible.

- **Typed tool lifecycle reducer seam (2026-08-05):** Added explicit
  `ToolStart`, `ToolUpdate`, and `ToolEnd` messages. The reducer addresses
  parallel rows by tool-call ID, updates grouped activity text, and appends
  output/result rows atomically. Actor coverage verifies out-of-order parallel
  completion; the next cutover is wiring `EventRenderer` tool events to these
  messages instead of the compatibility snapshot bridge.

- **ToolStart actor cutover (2026-08-05):** Production event-bus handling now
  derives and acknowledges `ScrollbackMsg::ToolStart` directly. The message
  carries the tool-call ID, semantic header, and grouped activity text, so the
  actor owns the canonical initial row even while update/end adapters remain
  transitional.

- **ToolUpdate actor cutover (2026-08-05):** Structured tool output and
  partial header updates now produce acknowledged `ScrollbackMsg::ToolUpdate`
  messages in the async event loop. Updates address rows by tool-call ID and
  no longer replace the actor snapshot wholesale.

- **ToolEnd actor cutover (2026-08-05):** Tool completion now emits an
  acknowledged `ScrollbackMsg::ToolEnd` carrying the final semantic header,
  activity completion text, error state, and output/result rows. The complete
  start/update/end lifecycle is now actor-routed; compatibility mutation is
  retained only as a transitional adapter until the renderer state maps are
  removed.

- **Tool compatibility-write removal (2026-08-05):** After actor startup,
  tool start/update/end handlers no longer mutate `Scrollback` rows, activity
  summaries, or output rows behind the compatibility mutex. They retain only
  lifecycle counters and buffers needed to construct the typed messages. The
  actor is now the sole production feed-row owner for the complete tool
  lifecycle.

- **Live snapshot-bridge removal (2026-08-05):** The production event loop no
  longer replaces the actor snapshot from compatibility state, and the YAML
  replay helper now uses acknowledged actor events directly. The transitional
  `ReplaceSnapshot` reducer message was removed entirely.

- **AgentStart status parity (2026-08-05):** Added the missing actor-owned
  `AgentStart -> Thinking` status projection. This closes the startup gap that
  was previously covered only by the compatibility renderer branch.

- **Full-gate audit (2026-08-05):** `just ci` now passes with the E2E
  transcript test reading the actor-owned snapshot. The remaining acceptance
  gap is deliberately explicit: `App` and compatibility `EventRenderer`
  constructors still retain `Arc<Mutex>` projections for synchronous YAML and
  focused reducer replay. Removing them requires moving that replay path to an
  acknowledged actor event sequence first; p23 remains in progress until that
  adapter is eliminated.

  The actor module now also pins this transition directly, independent of
  `EventRenderer` construction.

- **Actor-backed YAML visual replay (2026-08-05):** Added an acknowledged
  `EventRenderer::apply_actor_event` seam and switched the visual YAML runner
  to feed recorded events through `StatusActor`/`ScrollbackActor` snapshots.
  The first comparison caught and fixed a missing separator before the
  completion row; all 28 visual tests and YAML discovery now pass without
  snapshot weakening.

- **Actor-backed YAML state replay (2026-08-05):** Migrated the non-visual
  `run_scenario` replay helper to the same actor event seam and removed the
  final snapshot replacement adapter. YAML state and visual assertions now
  share event-derived actor projections.

- **Direct App actor ownership (2026-08-05):** `App` now creates and owns
  direct `StatusActor` and `ScrollbackActor` handles at construction. Renderer
  startup, YAML replay, E2E replay, snapshots, and app commands all reuse
  those handles without `Option`/mutex availability branching. Legacy
  `Arc<Mutex<Scrollback/StatusBar>>` fields remain only for compatibility
  widget constructors and focused synchronous renderer tests.

- **App mutex projection removal (2026-08-05):** Removed the legacy
  `Arc<Mutex<Scrollback>>` and `Arc<Mutex<StatusBar>>` fields from `App`.
  Actor renderer construction now receives only actor handles; compatibility
  mutex storage is confined inside the legacy `EventRenderer` test/replay
  constructor and is not part of application state. The full gate remains
  green.

- **Renderer projection encapsulation (2026-08-06):** Made both actor handles
  and legacy projection storage private to `EventRenderer`. External callers
  can no longer replace or mutate either projection behind the event-rendering
  boundary; production construction remains actor-backed through
  `with_actors`.

- **Projection storage separation (2026-08-06):** Replaced the renderer's
  unconditional compatibility mutex fields with an explicit private
  `Projection` enum. Actor-backed renderers now hold actor projections in those
  slots, while mutex storage exists only for legacy synchronous constructors;
  legacy access from an actor renderer fails loudly instead of silently
  creating a second source of truth. The compatibility adapter itself remains
  pending removal after synchronous replay is fully actorized.

- **UI command consumer cutover (2026-08-06):** The live binary now consumes
  `UiCommand` events emitted by `UiActor` for palette activation. It no longer
  dispatches session actions by rereading `UiState.last_palette_command`;
  command execution follows the actor event boundary.

- **Production projection isolation (2026-08-06):** Legacy
  `Arc<Mutex<Scrollback/StatusBar>>` storage and constructors are now compiled
  only for focused unit-test adapters. Production `EventRenderer` projection
  storage contains actor-backed slots only; the synchronous compatibility
  reducer remains available under test without widening the live architecture.

- **No default projection fallback (2026-08-06):** The production-only
  compatibility accessor now fails explicitly if reached instead of creating
  a default `Scrollback` or `StatusBar`. Actor-backed rendering remains the
  only live projection path; mutex adapters remain test-only.

- **Background lifecycle expansion (2026-08-06):** Cancellation and elapsed
  terminal states are carried as typed events and projected through the
  actor-owned feed, with YAML replay coverage.
Selection reset invariant (2026-08-06): `Scrollback::Clear` now clears the
actor-owned selected tool ID atomically with transcript rows, preventing a
post-Reset fold intent from targeting a stale block.

Status bus ownership (2026-08-06): the live `StatusActor` can now own a
subscription to the shared core event bus. `App` uses this constructor, so
the live renderer no longer mutates status projection while consuming feed
events; the deterministic YAML replay adapter retains its explicit
acknowledged reducer path for phase-locked scenarios.

Scrollback lifecycle ownership (2026-08-06): the live `ScrollbackActor` now
also owns the shared bus subscription for `Reset` and atomically clears its
transcript/selection projection. `App` uses this constructor; the renderer's
complex feed reducer remains the explicit deterministic replay seam until all
stateful tool grouping is moved into the actor.

Scrollback configuration ownership (2026-08-06): the same bus-owned actor now
projects `ThemeChanged` and `ToolDisplayModeChanged` into `ScrollbackMsg`
commands. Live renderer dispatch no longer owns these transcript settings;
focused actor tests pin reset and theme transitions.

Tool default ownership (2026-08-06): `ToolExecutionStart` now also projects
Grok's default collapsed/truncated display mode inside the live scrollback
actor. The deterministic replay path keeps its explicit mode command, while
the live renderer no longer writes this configuration directly.

Background lifecycle ownership (2026-08-06): background start/progress/
finish/cancel events now reduce to tool-card messages inside the live
`ScrollbackActor`. The renderer skips that pure event family in live mode;
the YAML replay reducer remains unchanged and independently testable.

Structured tool-output ownership (2026-08-06): active ordinary-tool updates
whose payload contains `output` or string `content` now reduce to `ToolOutput`
rows inside the live scrollback actor. Renderer-side buffering remains for
non-structured header updates and completion-card formatting; the actor test
pins multiline output projection.

Tool header-update ownership (2026-08-06): non-structured active-tool update
payloads now extend per-call header buffers inside `ScrollbackActor`, keyed by
tool ID. Live renderer update messages are no longer applied a second time;
completion-card and activity-summary reduction remain the next boundary.
