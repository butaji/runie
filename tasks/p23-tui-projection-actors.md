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

## Migration sequence

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

### Scrollback mutation inventory

The feed currently has distinct mutation families: append/clear, activity and
tool-row replacement, theme/tool-mode changes, reasoning/activity expansion,
timestamp and spacing normalization, and render-time live normalization. These
must become explicit reducer messages; a generic closure command is forbidden
because it would conceal ownership and make YAML event assertions impossible.
