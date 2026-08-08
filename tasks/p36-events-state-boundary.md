# p36 — Events are the state-transfer boundary

Status: in progress — live feed delivery consolidated; compatibility renderer retirement, strict color proof, and provider-boundary parity remain (2026-08-07)

Runie keeps mutable state inside its owning actor. Commands are actor-local
requests; durable state changes are transferred through `AgentEvent` (core)
or the actor's explicit message/event reducer (TUI). Renderers consume
snapshots only and never mutate another actor's state.

## Audit

- `AgentStateActor` reduces core events and publishes snapshots.
- `UiActor`, `PromptActor`, `StatusActor`, and `ScrollbackActor` own their
  models and publish immutable `watch` snapshots.
- `ScrollbackActor::new_with_bus` projects core events into feed events before
  reducing them; the renderer only reads the resulting `FeedSnapshot`.
- YAML replay already drives the same event path used by functional tests.
- Direct widget mutation is confined to actor workers (`PromptWidget` and the
  pure `FeedState` reducer); application code sends actor messages.

The detailed audit and per-change acceptance checklist are recorded in
[p48](p48-event-delivery-audit.md).

Pure projection extraction (2026-08-08): `ScrollbackActor` no longer imports
the renderer's `tool_header` formatter while reducing tool-start events. The
semantic header DSL now lives in `runie-tui-model`; the actor supplies its
workspace projection and emits model messages, while terminal rendering stays
in `runie-tui`. This removes a renderer → actor dependency without adding a
second state owner; existing actor and YAML tool-card replays remain the
behavior oracle.

Result-text extraction (2026-08-08): the same boundary now owns Pi tool-result
text normalization in `runie-tui-model`. `ScrollbackActor` no longer calls
`event_renderer::tool_result_text`; transport-envelope handling is reduced
before rendering, while the renderer retains only its compatibility helper.

Activity-label extraction (2026-08-08): grouped Grok activity text is now
projected by `runie-tui-model::activity_text` and consumed by the feed actor.
The model owns the semantic `◈ Listed/Read/Ran` vocabulary and failure suffix;
the renderer only styles the resulting line. Actor lifecycle tests preserve
the exact label and keep this event → snapshot behavior covered.

Classifier centralization (2026-08-08): `ActivityKind` and
`classify_activity_tool` now own the complete grouped activity alias vocabulary
in `runie-tui-model`; actor and compatibility-renderer counters consume the
shared classification, with unit coverage for every alias.

Completion-header extraction (2026-08-08): Grok's tool-result cardinality,
read-range/image suffixes, and terminal completion vocabulary now live in
`runie-tui-model::completed_tool_header_with_args`. The feed actor consumes
that projection directly, removing the final completion-format dependency on
`event_renderer` while preserving existing tool-card replay behavior.

Structured-update extraction (2026-08-08): streaming tool-update envelope
projection now lives in `runie-tui-model::structured_update_text` beside
`tool_result_text`. Both `ScrollbackActor` and `EventRenderer` consume the
model helper, so the partial-result `output`/`content` fallback and the
`None` envelope-skip behavior share a single renderer-independent
implementation. Focused tests cover the `output`-over-`content` precedence
and the non-string/`status`-only fallback; actor and renderer replay paths
still pin the event → snapshot contract.

Transport-only envelope predicate (2026-08-08): the
`status`-present-but-no-payload short-circuit shared by `ScrollbackActor`'s
update-projection filter and `EventRenderer`'s specialized card projection
now lives in `runie-tui-model::is_transport_only_update`. Both call sites
delegate the `status.is_some() && structured_update_text.is_none()` test
to the model helper, so the `{status: "running"}` skip behavior and the
`{status, output}` / unrelated-payload accept behavior share one
renderer-independent implementation. A focused unit test pins the
`status`-only true case alongside the `status + output` and `step: 2`
false cases; actor and renderer replay paths continue to gate partial
tool updates through the same event → snapshot contract.

Output-result classifier extraction (2026-08-08): the
`LineKind::ToolOutput` / `LineKind::ToolResult` decision for completed
tools now lives in `runie-tui-model::is_output_tool`, which owns the
`list_dir | list_files | read | read_file | web_fetch | web-fetch | fetch |
memory_search | memory-search` alias vocabulary. Both `ScrollbackActor` and
`EventRenderer` consume `runie_tui_model::is_output_tool(...)` so the
output-style vs result-style line kind is decided by a single
renderer-independent classifier. The actor and the compatibility renderer
now share the same alias set (including `memory_search | memory-search`),
so the structured-output rendering stays aligned across both event paths.
A focused unit test pins every alias and the negative `bash`/`subagent`
cases.

Boundary enforcement (2026-08-08): `validate-feed-actor-boundary.py` now
rejects renderer, Ratatui, and Crossterm imports from `ScrollbackActor` in
addition to direct widget reduction. This makes the declarative model/render
separation executable in CI rather than relying only on review discipline.

Reset lifecycle increment (2026-08-07): `AgentEvent::Reset` now maps to
explicit status/feed reset reducers. Status clears terminal turn facts while
preserving theme/context configuration, and feed clear resets turn-summary
eligibility. YAML and reducer tests cover the resulting event → snapshot
contract.

The runtime fixture `visual-reset-state.yaml` now replays a started reasoning
turn followed by `reset` and asserts the settled status, streaming flag, and
feed lifecycle bit without recompilation.

Prompt reset parity (2026-08-07): `PromptActor` now preserves its actor-owned
theme while rebuilding transient input state for `Reset`, matching the status
and feed actors' configuration-preserving reset behavior.

The same reset path preserves the actor-owned model caption; only transient
prompt editing state is rebuilt.

The YAML state oracle now supports explicit `thinking_elapsed_cleared` checks,
so reset fixtures can distinguish an omitted expectation from a required
`None` projection.

## Remaining work

1. Retire the compatibility `EventRenderer` state mirror after all replay
   callers are actor-backed; see [p47](p47-renderer-transient-state-migration.md).
2. Keep capture inputs declarative: the matrix accepts a scenario prompt, and
   `capture-scenario.sh` reads prompt/quit settings from the YAML fixture at
   runtime without recompilation.

**Welcome emission retirement (2026-08-08):** The renderer's `emit_welcome`
one-shot field and the `apply_actor_metadata` hook were the last
renderer-local state mirror held outside the actor mailbox. Both are gone,
and `EventRenderer::with_actors`/`with_live_actors` no longer take an
`emit_welcome` argument. The renderer's `run`/`apply_actor_event` paths now
emit only the actor-driven session-start lines on `AgentStart`. The
welcome modal is now composed by the YAML runner via
`replay_scenario_events` (and the visual replay helper), which pre-injects
`welcome_modal_lines()` into the `ScrollbackActor` mailbox when
`scenario.initial_prompt.is_none()`. The pure helper and its
`welcome_modal_snapshot` regression are retained. The first remaining
work item is therefore closed for the renderer surface; the live
capture-input keep-declarative contract and the strict color parity gap
remain separate. The full `just ci` (fmt-check, clippy, lint, test,
parity, source inventory, Pi event contract, and feed-actor boundary
validators) is green.

**Pre-injection helper relocation (2026-08-08):** The
`welcome_modal_lines()` helper and its `welcome_modal_snapshot` regression
now live in `widgets::welcome` rather than `event_renderer`. The YAML
runner and the renderer's pre-injection test both call it through
`crate::widgets::welcome_modal_lines()`, and the snapshot moved with the
test to `runie_tui__widgets__welcome__tests__welcome_modal.snap`. The
pre-injection event-to-state contract is unchanged: the idle chrome lines
still reach the `ScrollbackActor` mailbox before `AgentStart` when the
scenario omits an initial prompt, and the actor-owned scrollback still
reflects the same idle chrome a fresh session would emit. This is a
code-ownership cleanup only; the `ScrollbackMsg::Append` delivery path,
the renderer/actor boundary, and the actor snapshot consumer are
identical.

**Background completion helpers (2026-08-08):** `format_elapsed` and
`format_error` now live in `runie-tui-model::feed` next to the other
background-completion formatters. Both `EventRenderer` and
`ScrollbackActor` consume `runie_tui_model::{format_elapsed, format_error}`
when shaping the `Subagent completed/failed/cancelled` header, so the
elapsed suffix and parenthesised error fragment share a single
renderer-independent implementation. The renderer-local duplicates were
removed; focused unit tests cover the `None`/empty/`Some` and
`is_error` true/false branches.

**Tool-update header fragment (2026-08-08):** the streaming
`"{header} | update: {json}"` fragment is now produced by
`runie-tui-model::tool_update_header_text`. `EventRenderer`'s specialized
card projection and `ScrollbackActor::tool_update_messages` both call the
model helper instead of formatting the serialized `partial_result`
themselves, so the separator, the serialization, and the
`unwrap_or_default()` empty-fragment fallback share one
renderer-independent implementation. Focused unit tests pin the appended
JSON fragment (object and `null` payloads) and the empty-fragment shape;
the actor and renderer replay paths still gate partial updates through the
existing `is_transport_only_update` / `structured_update_text` contract.

**AgentEnd replay closure (2026-08-08):** the YAML replay path of
`EventRenderer::apply_actor_event` was inserting a phantom
`LineKind::Separator` row between `AgentStart` and `AgentEnd` whenever
`TurnStart` had preceded the closure. The live `run` branch at the bus
boundary never emitted that row, so deterministic replay fixtures
(`visual-reasoning`, `visual-error`, `visual-tool`, `visual-submitted`)
were carrying a stray blank cell that the live producer never produced.
The spurious `messages.push(ScrollbackMsg::Append(Line::new(LineKind::Separator, "")))`
in the `ActorEnd { .. } && turn_was_started` arm of `apply_actor_event`
was removed so the replay path now matches the `run` branch exactly:
session-start emits its own two wrapping Separator rows, `AgentEnd` emits
the `LineKind::TurnSummary` row and the navigation-only `TurnEnd`
message, and no extra transcript row is inserted between them. The
`actor_agent_end_emits_worked_for_only_after_turn_start` test was
tightened to assert the exact line count (4 with-TurnStart, 3 no-TurnStart)
and the exact `LineKind::Separator` count (2 in both cases, never 3),
which would have failed against the previous buggy code. The five
`visual_snapshots__visual-*.snap` files that exercised the Affirmations
fixtures were regenerated to drop the phantom blank row from the rendered
frames. The event-to-snapshot contract is now identical between the live
bus path and the YAML replay path, so the closure-evidence replay
fixtures no longer drift from a real captured session.

**BackgroundWork live/replay closure (2026-08-08):** the live
`EventRenderer::run` arm at the bus boundary used to drop every
`BackgroundWork*` event before delegating to `scrollback_messages_for_event`,
so the scrollback actor never received the `Subagent started/running/
completed/failed/cancelled` rows even though `apply_actor_event` and the
actor's own `bus_messages_for_event` adapter both projected them. The
filter
```rust
if matches!(event,
    AgentEvent::BackgroundWorkStarted { .. }
        | AgentEvent::BackgroundWorkProgress { .. }
        | AgentEvent::BackgroundWorkFinished { .. }
        | AgentEvent::BackgroundWorkCancelled { .. }) {
    Vec::new()
} else {
    scrollback_messages_for_event(&event)
}
```
was a stale relic from the period when the live `App` path double-subscribed
the scrollback actor to the bus and the renderer had to suppress one of the
two projections. After the live subscription consolidation in p36, the
scrollback actor is mailbox-only in production and the live `run` branch is
the single bus delivery boundary, so suppressing BackgroundWork* events only
deprived the transcript of the closure rows that the replay path already
emitted. The filter was removed so the live `run` branch now matches the
`apply_actor_event` replay arm at line 473 exactly:
`scrollback_messages_for_event(&event)` produces one `ToolStart` for
`BackgroundWorkStarted`, one `ToolUpdate` for `BackgroundWorkProgress`, and
a `ToolEnd` plus the optional `MarkToolError` for the finished/cancelled
variants. The `live_renderer_delivers_background_work_lifecycle_to_the_feed_actor`
test drives every variant through the live `run` branch and asserts the
matching `Subagent` row (started/running/completed/failed/cancelled), the
final `LineKind::ToolError` for the failed and cancelled closures, and the
exact single-row transcript state for the started variant. The paired
`live_and_replay_background_work_paths_produce_identical_rows` test drives
the same event sequence through both paths and compares the per-`work_id`
`(LineKind, text)` rows one work_id at a time, which would have failed
against the previous buggy code because the live path produced empty
transcripts while the replay path produced the full `Subagent` row set.
The BackgroundWork* event → transcript contract is now identical between
the live bus path and the YAML replay path.

The first two historical bullets are now closed. Production scroll, selection,
palette, and prompt transitions have named owner-local messages, and replay
assertions already support ordered `exact_events`, closed-contract `pi_events`,
and awaited `listener_events`. `visual-hey.yaml` exercises all three forms.
New scenarios should use these fields rather than adding bespoke assertions.

The matrix retains the original four-argument environment-assignment form;
the compatibility branch is covered by shell syntax/argument checks so older
capture recipes do not silently lose their color or parity-clock settings.

## Transition inventory audit (2026-08-06)

The externally observable TUI transitions are now represented by named
owner-local messages rather than implicit field writes:

| Transition family | Owner | Message/event boundary | Evidence |
|---|---|---|---|
| palette open/query/filter/escape/activate | `UiActor` | `UiMsg` | `visual-command-palette.yaml`, `ui.rs` reducer tests |
| welcome/shortcut visibility | `UiActor` | `UiMsg` | UI actor tests and visual YAML steps |
| prompt editing/submission/mode/search/theme | `PromptActor` | `PromptMsg` and core `AgentEvent` | prompt actor tests, event replay |
| feed append/update/fold/scroll/select/follow | `ScrollbackActor` | `ScrollbackMsg` | feed reducer tests and YAML state assertions |
| status/usage/theme/animation | `StatusActor` | `StatusMsg` and core/application events | status tests and visual matrix |
| session append/reset/restore/flush | `SessionActor` | session mailbox messages and bus events | session actor tests |

This closes the inventory item “unnamed TUI transition” for production actor
APIs. The remaining direct methods on `Scrollback` and `PromptWidget` are
reducer-local implementation details or compatibility constructors; they are
not called by the production actor boundary. The next architectural change is
therefore mechanical: migrate the remaining renderer-owned transient fields
using [p47](p47-renderer-transient-state-migration.md), then retire the
compatibility `EventRenderer` state mirror after replay callers have moved to
actor snapshots. It must not be replaced with another cross-actor mutation
path.

The separate strict color gap is tracked in p19/p25: the checked-in Grok cast
is symbol-exact but was captured with terminal-default SGR, while Runie emits
the selected Opaline theme tokens. `exact_attributes` must remain disabled for
that contaminated reference until a same-theme, same-terminal-mode paired
capture exists. This is an evidence gap, not permission to normalize colors or
claim attribute parity.

## Exhaustiveness hardening

Feed dispatch exhaustiveness increment (2026-08-08): the background and
actor-feed event tables now enumerate every intentionally ignored
`AgentEvent` variant instead of using wildcard no-op arms. Adding a new core
event therefore requires an explicit feed classification before the workspace
can compile.

UI reducer exhaustiveness increment (2026-08-08): command-palette and model-
selector subreducers now enumerate messages owned by other UI surfaces rather
than using wildcard fallthroughs. New `UiMsg` variants must be classified at
the owning reducer boundary.

Prompt actor event increment (2026-08-08): the prompt actor's subscribed core
event boundary now explicitly lists ignored lifecycle/session/tool/workflow
events and receiver errors; only reset, theme, and non-empty model changes can
mutate the prompt projection.

`status_messages_for_event` now names every intentionally ignored outer
`AgentEvent` variant and every intentionally ignored assistant sub-event.
This removes the wildcard fallback at the status boundary: adding a Pi event
or assistant sub-event now fails to compile until its status projection is
classified. The same exhaustive-table treatment should be applied to the
remaining feed and UI projection tables as their compatibility paths are
retired.

The UI reset mapper and actor-owned feed bus mapper now use the same explicit
classification. New core events must therefore be assigned to a projection,
or deliberately listed as a no-op, before the workspace compiles.

Live subscription consolidation (2026-08-07): `App` now constructs its
`ScrollbackActor` without a second bus subscription. `EventRenderer` is the
single interactive bus-delivery boundary and sends acknowledged feed reducer
messages to the actor. `ScrollbackActor::new_with_bus` remains available for
isolated actor integration tests and standalone projections, but it is not
used by the live app. This removes the possibility that one core event is
reduced twice by competing feed subscribers.

The compatibility renderer's feed adapter and the actor's background/workflow
adapter are explicit as well. This keeps the legacy replay path and the live
actor path aligned while the renderer is being retired as a state owner.

Renderer ownership re-audit (2026-08-07): a current source search confirms
that `EventRenderer::new`/`with_welcome` and direct legacy widget locks are
reachable only from `#[cfg(test)]` compatibility constructors and focused
tests. The live `App` path uses `with_live_actors`; YAML replay uses
`with_actors`. The remaining legacy adapter is documented migration debt, not
a second production state owner. New state changes must continue through the
actor message/event paths rather than extending that adapter.

The capture helper remains an external instrument, not production state. Its
bounded polling is intentionally limited to detecting terminal readiness and
settled output; it does not mutate Runie's state.

UI mailbox DSL closure (2026-08-06): the `UiActor` acknowledged-message path
now uses the shared `mailbox_ack!` expansion. This removes duplicated one-shot
plumbing while preserving the rule that every UI state transition enters via
an explicit `UiMsg` and is reduced by the owning actor before the caller
continues.

## Async ownership audit (2026-08-06)

All production task creation is owned:

- `LoopActor` stores the active loop `JoinHandle` until `wait_for_idle`/prompt
  completion consumes it.
- Core actor workers and event subscriber bridges retain `TaskOwner` handles.
- `App::spawn_renderer` returns the renderer `JoinHandle` to its caller.
- YAML recorder and pending-run tasks are joined before their scenario returns.
- The source lint rejects unannotated `tokio::spawn` sites; intentional test or
  orchestration sites carry an adjacent `OWNER` declaration.

This audit preserves the invariant that dropping an actor or scenario cannot
leave an orphaned task mutating shared state.

## Theme state assertion (2026-08-06)

The YAML state DSL now accepts `state.theme`, and `visual-theme-day.yaml`
verifies the actor-owned theme after the `ThemeChanged` event. Theme parity is
therefore observable as state as well as through rendered color cells.

## Pi model contract increment (2026-08-06)

Pi's `Model` exposes optional `samplingParams?: Record<string, unknown>`.
Runie now preserves this as `Model::sampling_params` at the serde boundary,
with a round-trip test proving the camelCase wire key and arbitrary JSON value
shape. Provider adapters can now receive the same model defaults as Pi.

The loop also merges those model defaults with per-request
`SimpleStreamOptions::sampling_params`, using request values as the winning
layer. The merge is pure and covered by a focused core test, keeping provider
configuration state inside the loop's owned option snapshot.

Pi's `timeoutMs` is now carried as `SimpleStreamOptions::timeout_ms` and
enforced by the async `HttpActor` boundary. Timeout cancellation is covered
with a pending-future test; no blocking sleep is used.

`maxRetries` is likewise carried as `SimpleStreamOptions::max_retries` and
implemented as bounded, actor-local retry attempts around the async transport.
The deterministic flaky transport fixture proves two failures followed by a
successful third attempt, without sleeps or detached tasks.

The TUI replay schema now exposes `provider_options` for
`timeout_ms`/`max_retries`/`sampling_params`; the `visual-hey.yaml` fixture
uses the sampling field, proving these provider settings can be edited and
replayed from YAML without recompiling the runner. Its `assertions` block now
also verifies the effective options received by the provider stream, rather
than merely validating YAML deserialization.

## Direct-mutation audit (2026-08-06)

The next unresolved event boundary is recorded in p38: LoopActor control
fields (`running`, active-run ownership) and the legacy
renderer adapter still have private mutation paths. Queue contents and the
actor-owned FeedState and queue modes already satisfy the event boundary. p38 preserves the
closed Pi event contract while introducing Runie application control events
and snapshot-only compatibility rendering.

State mailbox acknowledgement closure (2026-08-06): `ReplaceMessages` now
carries an explicit one-shot acknowledgement. `LoopActor::replace_messages`
and YAML session-context restore therefore return only after
`AgentStateActor` has reduced the replacement, eliminating a
scheduler-dependent state race.

Queue acknowledgement closure (2026-08-06): steering and follow-up `push`
and `clear` commands now use the shared `mailbox_ack!` DSL. Queue callers
observe completion only after the owning reducer has inserted or removed the
messages, while drain/length operations retain their existing reply path.

Provider acknowledgement closure (2026-08-06): `ProviderActor::cancel` now
waits for the owned worker to abort its `JoinSet` pumps before returning.
Cancellation callers therefore observe a settled provider boundary rather
than merely an enqueued cancel command.

Configuration exception audit (2026-08-06): the Pi-compatible
`set_default_stream_fn` singleton remains an explicit provider configuration
API, not a live agent/TUI state projection. Production `LoopActor` instances
receive their `StreamFn` through `ProviderActor::new`; no production state
transition uses the singleton, so it is not a second runtime state owner.

State mailbox DSL consolidation (2026-08-06): acknowledged `StateCommand`
construction now shares `mailbox_ack!` plumbing through the state actor's
private helper. State ownership and command payloads remain explicit at each
public method.

TUI theme event boundary (2026-08-06): `App::set_theme` publishes one
`ThemeChanged` event. Prompt remains bus-reactive; status and scrollback are
reduced by the single production `EventRenderer` bus-delivery boundary, so
the app does not create a second competing projection subscription.

Session mailbox consolidation (2026-08-06): journal append/reset/import/flush
and the session event projection now acknowledge through `mailbox_ack!`, so
session state remains actor-owned while all mutation entry points share the
same event/mailbox DSL boundary.

Prompt mailbox consolidation (2026-08-06): prompt clear/mode/theme/caption/
search/event mutations now use the shared acknowledgement DSL, while key
handling retains its typed outcome reply. No prompt widget state crosses the
actor boundary directly.

Loop control consolidation (2026-08-06): steering/follow-up mode and run
lifecycle commands share the acknowledgement DSL, preserving the loop actor
as the sole owner of control state and keeping event reduction observable.

Model configuration event boundary (2026-08-06): Runie-only `ModelChanged`
events now carry model selection from `LoopActor` through the core state,
status, and prompt actors. The event is explicitly excluded from the Pi wire
contract, while `App::refresh_model_caption` no longer mutates projections
directly.

YAML replay boundary (2026-08-06): declared context-window settings now enter
the status projection through `ModelChanged`, eliminating the remaining direct
status mutation in scenario replay.

State event application consolidation (2026-08-06): `AgentStateActor` now
acknowledges `ApplyEvent` through its shared mailbox DSL, preserving the
single actor-owned event-to-state boundary.

Mailbox ownership audit (2026-08-06): core and TUI production unit-ack
commands have no remaining handwritten oneshot plumbing; typed response
commands remain explicit because their result values cannot use the unit-ack
DSL.

Provider cancellation consolidation (2026-08-06): cancellation acknowledgement
now uses `mailbox_ack!`, retaining provider pump ownership and the settled
abort boundary without duplicating oneshot plumbing.

Continuation audit (2026-08-07): the remaining `LoopActor` mutex protects only
the actor-owned in-flight `JoinHandle` used by `wait_for_idle`; it is
coordination state, not a domain snapshot or cross-actor projection. Run
lifecycle transitions still enter through acknowledged `LoopControlEvent`
messages, and the run task is awaited before completion. Remaining production
`tokio::spawn` sites are actor-worker macro expansions, an actor-owned provider
pump, or renderer workers whose owners are retained and shut down. No new
direct state-transfer mutation was found in this audit.

Coordination-state classification (2026-08-07): the remaining production
`Mutex`/spawn sites were checked against the SSOT rule. `LoopActor::current` is
an owner-local `JoinHandle` slot used only to await an in-flight run;
`SubscriberRegistry` protects registration ordering; transport/replay mutexes
are observation probes. The only renderer state mirror remains the
test/replay-only `Projection::Legacy` adapter. This classification does not
close p36/p47: compatibility retirement and full deterministic cast parity
remain explicit acceptance work.

Live/replay Grok separator closure (2026-08-08): `EventRenderer::run` now
removes the separator at `feed_messages.get(1)`, after the intentional
`AssistantStreamStart` marker, while `apply_actor_event` retains the pure
five-message projection. The parity test drives `MessageStart::Assistant`
through `with_live_actors`/live bus and `with_actors`/replay, asserting four
live rows versus five replay rows; the pure-table oracle remains at five.

**Immediate-render key dispatch (2026-08-08):** the `runie` binary's
`run_app` loop used to push every Press key into a `pending_keys`
`VecDeque` from the input arm and then drain the queue inside the 50 ms
`tick` arm, awaiting each key through the prompt/UI actor in order and
drawing a single terminal frame only after the last queued key had been
reduced. A fast burst like `Hey` only became visible after five
sequential mailbox awaits plus one render, so the first character
lagged the typist by tens of milliseconds and burst keystrokes could
appear in one frame. The input arm now drops the FIFO entirely:
`tokio::select!` uses `biased;` to prefer the input branch, and every
`InputEvent::Key(key)` is dispatched immediately through a new
`dispatch_key` inner helper that moves the `ui_commands` broadcast
receiver in and returns it back across the await. After the dispatch
returns, the input arm calls a new `render_frame` helper (placeholder
visibility update + the same `terminal.draw` block that the tick arm
used to run) so a single key produces a single frame. The tick arm is
now render-only and covers animation and agent activity refreshes.
Mouse events, the model-selector and command-palette routing, the
`map_key` action table, the mappable-builtin and `is_quit_command`
prompt-submit paths, the renderer-shutdown teardown for the `Quit`
action, and the bias for input over tick are all preserved. The
existing `full_mode_typed_prompt_snapshot` (`crates/runie-tui/tests/
e2e/visual-typed.yaml`) and 27 sibling visual snapshots still pass, and
a new `prompt_actor_reduces_each_press_key_independently` unit test
in `crates/runie-tui/src/bin/runie.rs` drives `handle_key` for the
five `hello` characters and asserts the prompt text contains the
growing prefix after every individual `await`, so a future regression
that batches keys through a single awaited reducer would surface as
a stale snapshot. The full `just ci` (fmt-check, clippy, lint, test,
parity, source inventory, Pi event contract, feed-actor boundary) is
green.

**Prompt/Ui mailbox bias (2026-08-08):** `run_prompt_actor` and the
`UiActor` worker both used a `tokio::select!` between a `mpsc` mailbox
and a `broadcast::Receiver` without `biased;`. When the agent was busy
publishing `MessageUpdate`/`ToolExecution*`/`BackgroundWork*` events
the broadcast receiver was almost always `Ready`, so `tokio` picked the
event branch pseudo-randomly even when a freshly pressed key was
already queued on the mailbox, adding tens of milliseconds of
per-keystroke latency. Both `select!`s now lead with `biased;` so the
mailbox branch is checked first whenever a key/UI message is ready.
The new `prompt_actor_services_key_mailbox_before_draining_queued_events`
test drives `run_prompt_actor` directly with 16,384 queued
`AgentStart` events plus a pre-positioned key in the mailbox; the test
parks the actor on a `(Notify, Notify)` pair right after the key
reducer runs and asserts `event_counter == 0`, which fails by 2+ events
when `biased;` is removed and passes deterministically with the fix.
The actor's `event_counter` and the pause hooks are `cfg(test)`-gated
so the production code path keeps the original four-argument
`run_prompt_actor` signature and pays no allocation cost per event.
The full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) is green.

**Render-frame snapshot/await reduction (2026-08-08):** `runie` now caches
placeholder visibility in the event loop and only sends the prompt actor
mailbox update when settled/idle state transitions. Each frame captures one
`TuiSnapshot` before `terminal.draw`, reuses one `PromptWidget` for both cursor
position and rendering, and reads session state only when the session-info
overlay is open. Key dispatch remains paired with exactly one immediate
`render_frame` call; focused binary/library tests and `just ci` cover the
change.

**UiActor mailbox-bias regression cover (2026-08-08):** the `biased;`
above was only pinned by a test on the prompt side; the `UiActor` worker
carried the same bias with no executable proof, so removing it would have
regressed palette/shortcut latency silently. The worker body was extracted
out of the `spawn_actor_worker!` closure into a free
`run_ui_actor(rx, events, snapshot_tx, command_tx, initial, event_counter,
#[cfg(test)] pause_hooks)` mirroring `run_prompt_actor`, and `UiActor`
gained the same `#[allow(dead_code)]` `event_counter: Arc<AtomicUsize>`
field. The counter is incremented in the broadcast branch, and the mailbox
branch parks on the test-only `(Notify, Notify)` pair after the `UiMsg` has
been reduced, published, and acknowledged. Production construction passes
`None`, so the pause and the sixth/seventh arguments compile out of the
release path. The new
`ui_actor_services_mailbox_before_draining_queued_events` test drives
`run_ui_actor` directly with 16,384 queued `AgentStart` events (which map
to no `UiMsg`, so they cost a match arm rather than snapshot churn) and
eight pre-queued `UiMsg::ToggleCommandPalette` messages, then asserts
`event_counter == 0` at each of the eight pause points plus the
`command_palette_open` snapshot after the first reduction. Eight
observation points rather than one is deliberate: with a single message the
unbiased `select!` picks the mailbox branch by chance about half the time,
and a measured 25-run sweep of the deleted-`biased;` build caught the
regression only 15/25 times; the eight-point form caught it 25/25 while
staying deterministic (10/10) with the fix in place. The mailbox typedef is
now the named `UiMailbox` alias so the worker signature and the test share
one spelling of the acknowledged-message tuple.

**PromptActor mailbox-bias regression parity (2026-08-08):** the
existing `prompt_actor_services_key_mailbox_before_draining_queued_events`
test pinned the prompt `biased;` with a single key and a single observation
point, which left the prompt half of the same coin-flip hole the UiActor
entry above just closed. With one pre-queued message the unbiased
`select!` picks the mailbox branch about half the time, and a measured
25-run sweep of the deleted-`biased;` build caught the prompt regression
only 15/25 times; the eight-point form caught it 25/25 while staying
deterministic (10/10) with the fix in place. The test now mirrors the
UiActor counterpart: it lifts the message count to
`const MESSAGES: usize = 8`, resizes the mailbox channel to `MESSAGES`,
loops over eight distinct `KeyEvent` keys (cycling `x` / `y` / `z` so the
buffer never spells `/history` mid-test), collects the
`oneshot::Receiver<PromptOutcome>` into a `Vec`, and waits on
`key_done.notified().await` at every one of the eight pause points while
asserting `event_counter == 0` and then calling `actor_release.notify_one()`
to unblock the actor for the next reduction. The first reduction also
asserts `snapshot_rx.borrow().text == "x"` so the published snapshot is
proven to reflect the typed character alongside the counter invariant.
After all eight pauses the test drains the reply `Vec` and asserts every
`PromptOutcome::Edited`, so a regression that drained events before keys
now fails both at the per-iteration counter check and at the final reply
shape. The docstring on the test records the coin-flip concern and the
25-run sweep measurement so the next reader doesn't shrink it back to
one. The full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) is green.

**Input actor terminal-event bias (2026-08-08):** the owned input actor in
`runie` ran a three-arm `tokio::select!` between `input.next()` (terminal
events), `input_config_rx.recv()` (config from the main loop), and
`cadence.tick()` (16 ms scroll-flush cadence) without `biased;`. When all
three branches were ready — which happens whenever a keystroke lands in
the same scheduler slice as a cadence tick — `tokio` picked one at random,
so a fast typist could see a keystroke delayed by one cadence window of
`input.next()` work and the loss felt worst while the agent was streaming
(the cadence branch was almost always ready). The select now leads with
`biased;` so the terminal-event arm is checked first, then config updates,
then cadence; this matches the outer `run_app` loop, which already
prioritized input via `biased;` and immediate `render_frame`. The input
channel capacity was also raised from 32 to 128 so the actor can absorb
transient render-time backpressure (the receiver briefly stalls past
~100 ms during heavy agent-streaming frames) without forcing
`input_tx.send(...).await` to block and drop a keystroke. No new state
owner, no second subscription path: terminal events remain the single
producer of `InputEvent` to the mailbox. The four `runie` binary unit
tests, the 28 `visual_snapshots` replay tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

**Tool-event mailbox hop coalescing (2026-08-08):** the live `run` path
and the replay `apply_actor_event` path in
`crates/runie-tui/src/event_renderer.rs` previously crossed the
`ScrollbackActor` mailbox twice per `ToolExecutionStart` / `Update` / `End`
event: once for the `Set*` / activity rows from
`scrollback_messages_for_event`, then again for the specialized
`ToolStartRunning` / `ToolUpdate` / `ToolEnd` row produced by
`handle_tool_*`. Each hop published its own `FeedSnapshot` and forced the
renderer to wait for a second reduction before the bus loop could move
on. Both paths now append the specialized tool message to `feed_messages`
before the single `apply_batch` so every tool lifecycle event produces one
mailbox round-trip and one snapshot publication. Message order is
preserved by `scrollback_messages_for_event`: tool start already emits
`SetToolName`, `SetToolArgs`, `ActivityToolStart`, `SetToolMode` before
the appended `ToolStartRunning`; tool end already emits `ActivityToolEnd`
and `RemoveToolArgs` before the appended `ToolEnd`; tool update has no
rows in `scrollback_messages_for_event` so the appended `ToolUpdate` is
the only message in the batch. The focused regression
`live_renderer_delivers_tool_updates_to_the_feed_actor` now also
publishes `ToolExecutionEnd` and asserts the block is no longer running.
The 219 `runie-tui` unit tests, the 28 `visual_snapshots` replay tests,
and the full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) are green.

**Long-burst prompt reducer regression (2026-08-08):** the
`prompt_actor_reduces_each_press_key_independently` 5-char burst
covered the snapshot-staleness contract but left a long-burst gap: a
regression that quietly dropped characters (or coalesced keys) would
have surfaced only as a wrong `text == expected` final assertion on a
short input. The new
`prompt_actor_reduces_one_hundred_twenty_eight_keys_into_snapshot`
unit test in `crates/runie-tui/src/bin/runie.rs` mirrors the sibling
`App`/`PromptActor` wiring, drives 128 distinct char keys (26
lowercase + 26 uppercase + 10 digits + 66 symbols) through
`app.prompt.handle_key(key).await`, asserts after every awaited
dispatch that `app.prompt.snapshot().text()` ends with the cumulative
prefix, and finally asserts the verbatim equality of the snapshot
against the concatenated input. The five `runie` binary unit tests,
the 220 `runie-tui` unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**`FinalizeAssistant` snapshot-clone dedup (2026-08-08):** both the live
`EventRenderer::run` and the replay `apply_actor_event` paths in
`crates/runie-tui/src/event_renderer.rs` were calling
`self.thinking_elapsed_ms()` twice in adjacent fields of the
`ScrollbackMsg::FinalizeAssistant { summary, settled_no_tool_phase }`
push, and each call walked `self.status_actor.model_snapshot()` and
cloned the full `StatusSnapshot` just to read one `Option<u64>`. The
two call sites now bind `let thinking_elapsed_ms = self.thinking_elapsed_ms();`
immediately before the push and use the cached `Option<u64>` in both
fields, so each `MessageEnd::Assistant` closure crosses the status
snapshot once instead of twice. The `FinalizeAssistant` payload shape
and the `thinking_summary` projection are unchanged, so the
`event_renderer::tests` module (25 tests, including
`actor_message_update_appends_text_to_assistant_line`,
`live_and_replay_assistant_start_preserve_layout_parity_contract`, and
`actor_agent_end_emits_worked_for_only_after_turn_start`) still pins
the same event → transcript contract. The 25 `event_renderer::tests`
unit tests, the 28 `visual_snapshots` replay tests, and the full
`just ci` (fmt-check, clippy, lint, test, parity, source inventory,
Pi event contract, feed-actor boundary) are green.

**`handle_tool_end` atomic snapshot coalesce (2026-08-08):**
`EventRenderer::handle_tool_end` in `crates/runie-tui/src/event_renderer.rs`
walked `self.scrollback_actor.model_snapshot()` four times — once each
through `active_tool_count`, `activity_counts`, `current_tool_header`,
and `current_tool_args` — and each call cloned the full `FeedSnapshot`
just to read a few fields. A concurrent `ScrollbackActor` `apply_batch`
between any two reads could leave the tool-end card header, args,
activity counts, and active-tool count disagreeing about which tool was
closing. The four helpers are now free functions taking
`snapshot: &FeedSnapshot` (placed at module scope right after the
`impl EventRenderer` block) and `handle_tool_end` binds one
`let snapshot = self.scrollback_actor.model_snapshot();` at the top of
its body, threading `&snapshot` through every helper call so the card
header, args, activity text, and active-tool count all observe the
same atomic `FeedSnapshot`. The clippy `too_many_lines` allow reason
now states the atomic-snapshot guarantee. The other three call sites
of the converted helpers each get their own local snapshot read — no
further consolidation: `handle_tool_start` reads once for
`active_tool_count`, `handle_tool_update` reads once for
`current_tool_header` (the earlier running-block check at the top of
the function is a separate read), and `activity_counts_with_start`
reads once for its internal `activity_counts` call. `activity_group_exists_since_latest_user`
and `activity_counts_with_start` stay as `&self` methods. A new
`use runie_tui_model::FeedSnapshot;` import backs the free-function
signatures. The 219 `runie-tui` lib unit tests, the 5 `runie` binary
unit tests, the 28 `visual_snapshots` replay tests, and the full
`just ci` (fmt-check, clippy, lint, test, parity, source inventory,
Pi event contract, feed-actor boundary) are green.

**`handle_tool_start`/`handle_tool_update` atomic snapshot coalesce
(2026-08-08):** `EventRenderer::handle_tool_start` and
`EventRenderer::handle_tool_update` in
`crates/runie-tui/src/event_renderer.rs` each walked
`self.scrollback_actor.model_snapshot()` more than once. `handle_tool_start`
called `active_tool_count(&self.scrollback_actor.model_snapshot())` and
`self.activity_group_exists_since_latest_user()` (which itself walked
`self.scrollback_actor.model_snapshot()`) and then
`self.activity_counts_with_start(...)` (which walked
`self.scrollback_actor.model_snapshot()` again via its internal
`activity_counts` call), for three FeedSnapshot reads where a single
intervening `apply_batch` could disagree about whether the new tool
started a fresh activity group. `handle_tool_update` called
`self.scrollback_actor.model_snapshot()` for the running-block check
and then again for `current_tool_header(&self.scrollback_actor.model_snapshot(), ...)`,
so the running-block predicate and the header text could disagree about
the same tool-call_id across two reads. The two `&self` helpers
`activity_group_exists_since_latest_user` and `activity_counts_with_start`
are now free functions taking `snapshot: &FeedSnapshot` (placed at module
scope right after the existing `activity_counts` helper), matching the
free-function pattern already used by `current_tool_header`,
`current_tool_args`, `active_tool_count`, and `activity_counts`. Each
function binds one `let snapshot = self.scrollback_actor.model_snapshot();`
at the top of its body and threads `&snapshot` through every helper
call, so the activity-grouping flag, the activity counts, the
running-block predicate, and the current-tool header all observe the
same atomic `FeedSnapshot`. The clippy `too_many_lines` allow reasons on
both `handle_tool_start` and `handle_tool_update` now state the
atomic-snapshot guarantee. The full `just ci` (fmt-check, clippy,
lint, test, parity, source inventory, Pi event contract, feed-actor
boundary) is green.

**`MessageEnd::Assistant` atomic snapshot coalesce (2026-08-08):** the live
`EventRenderer::run` bus arm and the replay `EventRenderer::apply_actor_event`
path in `crates/runie-tui/src/event_renderer.rs` each walked
`scrollback_actor.model_snapshot()` twice per event: once for
`let turn_was_started = scrollback_actor.model_snapshot().turn_started;` and
again inside the `AgentEvent::MessageEnd { message: AgentMessage::Assistant(_) }`
branch for `let feed_snapshot = scrollback_actor.model_snapshot();`, which
supplies `has_reasoning` (a `LineKind::Reasoning` scan over
`feed_snapshot.lines`), `reasoning_expanded`, and the
`feed_snapshot.tool_blocks.is_empty()` half of `settled_no_tool_phase`. Each
call clones the entire `FeedSnapshot` — `lines`, `tool_blocks`, `tool_names`,
and the navigation projection — out of the actor's `watch` cell
(`ScrollbackActor::model_snapshot` at `crates/runie-tui/src/scrollback_actor.rs:110`
is `self.snapshot.borrow().clone()`), so an assistant closure paid two full
clones and, worse, could observe two different scrollback generations: an
`apply_batch` landing between the two reads would let the turn-summary
eligibility flag disagree with the reasoning/tool-block facts used to build
`ScrollbackMsg::FinalizeAssistant`. Both paths now bind one
`let scrollback_snapshot = scrollback_actor.model_snapshot();` at the top of
the per-event projection body — immediately before
`scrollback_messages_for_event(&event)` — read `turn_was_started` from that
cached value, and reuse it in the `MessageEnd::Assistant` branch via
`let feed_snapshot = &scrollback_snapshot;`, so `turn_was_started`,
`has_reasoning`, `reasoning_expanded`, and `settled_no_tool_phase` all observe
the same atomic `FeedSnapshot`. The clippy allow reasons on both functions now
state that guarantee. Placement is deliberate: the binding sits at the start of
the feed-message-building block rather than the literal first statement of the
function. In `run` the literal top is outside the `loop`, which would freeze one
snapshot for the whole session; in both functions the literal top also precedes
`status_actor.apply_event(&event).await`, and moving a scrollback read backwards
across an await point would widen — not close — the window in which a concurrent
mailbox writer can change the feed. Everything from the new binding to the
`FinalizeAssistant` push is synchronous, so the coalesced read is exactly
equivalent to the previous first read and strictly narrower than the previous
second one. The `FinalizeAssistant` payload shape and message ordering are
unchanged. The 25 `event_renderer::tests` unit tests (including
`actor_agent_end_emits_worked_for_only_after_turn_start`,
`actor_message_update_appends_text_to_assistant_line`, and
`live_and_replay_assistant_start_preserve_layout_parity_contract`), the 219
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check, clippy,
lint, test, parity, source inventory, Pi event contract, feed-actor boundary)
are green.

**Thinking summary helper relocation (2026-08-08):** the Grok "◆ Thought for X.Xs"
label and its 900 ms fallback duration now live in
`runie-tui-model::feed::thinking_summary` and
`runie-tui-model::DEFAULT_THINKING_ELAPSED_MS`. Both call sites in
`crates/runie-tui/src/event_renderer.rs` (`run` at line 371 and
`apply_actor_event` at line 505) consume the model helper via
`pub use runie_tui_model::thinking_summary;` placed next to the existing
`pub use runie_tui_model::status_messages_for_event;` re-export, so the
`ScrollbackMsg::FinalizeAssistant { summary, .. }` payload keeps its exact
"◆ Thought for 0.9s" default and "◆ Thought for N.Ns" observed shape
without keeping a renderer-local copy. The renderer-local constant and
function were removed. A focused unit test
`thinking_summary_pins_default_and_observed_elapsed` pins both the
`None → DEFAULT_THINKING_ELAPSED_MS` fallback and the observed
`Some(2_500) → "◆ Thought for 2.5s"` projection. The 25
`event_renderer::tests` unit tests, the 65 `runie-tui-model` lib unit
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

Prompt-timestamp extraction (2026-08-08): `runie-tui-model::feed` now owns
the live-prompt short-clock projection. `ScrollbackActor` no longer
references the renderer's `LIVE_TIMESTAMP_SECONDS_MIN`,
`format_clock_timestamp`, or `local_clock_parts`; the renderer-side copies
were deleted and the `libc = "0.2"` dependency moved with them to
`runie-tui-model`. `ScrollbackActor::scrollback_messages_for_event` now
compares `user.timestamp` against the model-owned
`PROMPT_TIMESTAMP_LIVE_THRESHOLD` and projects the user-message wall clock
through `runie_tui_model::format_clock_timestamp`, so the model remains
the single owner of the `H:MM AM/PM` shape and the UTC-derived 12-hour
fallback. A focused unit test
`format_clock_timestamp_pins_short_clock_shape` pins the short-clock
shape across midnight, the 12-hour rollover, and the single-digit minute
zero-padding, so replay and live paths share one identity. The 66
`runie-tui-model` lib unit tests (including the new
`format_clock_timestamp_pins_short_clock_shape`), the 218 `runie-tui` lib
unit tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

Workflow-row formatter relocation (2026-08-08): the Grok "Workflow name:
objective" transcript projection now lives in
`runie_tui_model::feed::workflow_text`. The four renderer-local test-only
duplicates — `format_elapsed`, `format_duration`, `workflow_phase_mark`, and
`workflow_text` at `crates/runie-tui/src/widgets/scrollback.rs:27-95` —
were removed; the three call sites in `#[cfg(test)] fn Scrollback::apply_legacy`
at lines 427, 462, and 488 (`ScrollbackMsg::WorkflowStart`,
`ScrollbackMsg::WorkflowProgress`, `ScrollbackMsg::WorkflowEnd`) now call
`runie_tui_model::workflow_text(header, phases, status, elapsed_ms, active_agents)`,
and the six focused assertions in
`workflow_card_uses_grok_status_and_phase_glyph_order` and
`workflow_objective_flattens_multiline_source_text` invoke the same
canonical helper. The model-side test
`workflow_phase_glyphs_match_grok_fallback_for_terminal_states` was renamed
in place from `super::workflow_text_model` to `super::workflow_text`, and
`pub use runie_tui_model::workflow_text;` was added to
`crates/runie-tui-model/src/lib.rs` next to the existing
`thinking_summary` re-export. Signatures match exactly
(`header: &str, phases: &[(String, String)], status: &str, elapsed_ms: Option<u64>, active_agents: u32`),
so the call sites needed no parameter rewrites. The 66 `runie-tui-model`
lib unit tests, the 218 `runie-tui` lib unit tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

Renderer-local `tool_result_text` retirement (2026-08-08): the Pi
tool-result envelope → user-visible-text normalizer now has a single
owner. The renderer-local `pub(crate) fn tool_result_text` at
`crates/runie-tui/src/event_renderer.rs:1096-1133` (the trailing comment
documenting Pi's `content: []` zero-row card semantics and the
`output`/`error` fallback chain) was removed; the four call sites at
lines 692 (`handle_tool_end` raw-output projection), 710 (web-search
sources line), 952 (`completed_tool_header` cardinality formatting), and
1069 (`completed_tool_header_with_args` read-range line count) now call
`runie_tui_model::tool_result_text` directly. The four focused assertions
in `event_renderer::tests::structured_tools_use_grok_headers_and_preserve_output_rows`
at lines 2265, 2267, 2271, and 2275 (string passthrough, `output`
fallback, empty `content: []` zero-row card, and `error` fallback after
empty content) now exercise the model helper, so the renderer no longer
keeps a duplicate projection of the same protocol envelope. No model
changes were needed: `runie_tui_model::tool_result_text` already lives at
`crates/runie-tui-model/src/feed.rs:151-182` and is re-exported via
`crates/runie-tui-model/src/lib.rs`, so the renderer call sites needed
no signature rewrites and the actor-side behavior stays identical
because `ScrollbackActor` was already reducing through the model helper.
The 66 `runie-tui-model` lib unit tests, the 218 `runie-tui` lib unit
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

`PromptWidget::cycle_mode` owner retirement (2026-08-08): the renderer
no longer owns the input-mode rotation policy. The Body of
`PromptWidget::cycle_mode` at `crates/runie-tui/src/widgets/prompt.rs:151-159`
was collapsed to `self.mode = runie_tui_model::cycle_input_mode(self.mode);`,
and the new canonical helper `pub fn cycle_input_mode(mode: InputMode) -> InputMode`
now lives at `crates/runie-tui-model/src/prompt.rs` next to `InputMode`,
matching the existing model/view split used by `feed.rs` and `status.rs`.
`pub use runie_tui_model::cycle_input_mode;` was added to
`crates/runie-tui-model/src/lib.rs` next to the existing `InputMode` /
`PromptOutcome` / `PromptSnapshot` re-export, so the widget's existing
`pub use runie_tui_model::{InputMode, PromptOutcome, PromptSnapshot};` at
`crates/runie-tui/src/widgets/prompt.rs:19` now reaches the same canonical
helper through that re-export. The widget-side assertion
`mode_cycles_through_normal_alternate_and_plan` at
`crates/runie-tui/src/widgets/prompt.rs:619-629` already exercises the
public `cycle_mode` path end-to-end, so the model-side coverage stays
narrow and focused: the new test `cycle_input_mode_pins_trio_and_file_self_loops`
in `crates/runie-tui-model/src/prompt.rs` covers the four call points —
the Normal → Alternate → Plan → Normal triple rotation, the
`FileSearch → FileSearch` and `FileViewer → FileViewer` self-loops (the
async actors own entry/exit for file modes, so the cycle must pin them
rather than rotate them), and one snapshot round-trip that carries a
cycled mode through `PromptSnapshot::default()` to confirm the
projection agrees. The widget test (`mode_cycles_through_normal_alternate_and_plan`),
the new model test (`cycle_input_mode_pins_trio_and_file_self_loops`),
the 66 `runie-tui-model` lib unit tests, the 218 `runie-tui` lib unit
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

Renderer-local `activity_text` retirement (2026-08-08): the Grok
grouped tool activity label now has a single owner. The renderer-local
`pub(crate) fn activity_text` at `crates/runie-tui/src/event_renderer.rs:1179-1221`
(the `dirs`/`files`/`commands`/`subagents`/`failures`/`running`
projection with the `◈` prefix and `Listing`/`Listed`/`Reading`/`Read`/
`Running`/`Ran` verb alternation) and its helper
`fn append_failure_suffix` at `crates/runie-tui/src/event_renderer.rs:1223-1228`
(append the `· N failed` suffix only when `failures > 0 && !running`)
were removed; the two call sites at lines 582 (`handle_tool_start`
running activity, `running = true`) and 679 (`handle_tool_end`
settled activity, `running = false`) now invoke
`runie_tui_model::activity_text` directly via the new
`pub use runie_tui_model::activity_text;` re-export placed next to
`status_messages_for_event` and `thinking_summary` at
`crates/runie-tui/src/event_renderer.rs:16`. The six focused assertions
in `event_renderer::tests::activity_group_labels_match_grok_rich_recording`
at lines 2365, 2369, 2372, 2374, 2378, and 2382 (the running pair
`Listing 1 dir, Reading 1 file` / `Listed 1 dir, Read 1 file`, the
plural `Listed 2 dirs`, the cross-family `Listed 1 dir, Ran 1 command`,
the `Ran 2 commands · 1 failed` failure suffix, and the heterogeneous
`Read 1 file, Ran 1 subagent`) now exercise the model helper, so the
renderer no longer keeps a duplicate projection of the same Grok
activity vocabulary. No model changes were needed: `runie_tui_model::activity_text`
already lives at `crates/runie-tui-model/src/feed.rs:189-231` and is
re-exported via `crates/runie-tui-model/src/lib.rs`, so the renderer
call sites needed no signature rewrites and the model-side test
`activity_group_labels_match_grok_rich_recording` already lives next
to the canonical helper. The 67 `runie-tui-model` lib unit tests,
the 218 `runie-tui` lib unit tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

Web-search summary projection centralization (2026-08-08): the Grok
web-search summary projection now has a single owner. The renderer-local
`fn web_search_site_count` at `crates/runie-tui/src/event_renderer.rs:1103-1126`
and `fn web_search_sources_line` at
`crates/runie-tui/src/event_renderer.rs:1128-1163` were removed; the
two call sites at `crates/runie-tui/src/event_renderer.rs:710-715`
(successful `web_search` / `web-search` completion row appended to the
`ToolEnd` output, built from `runie_tui_model::tool_result_text(&result)`)
and `:981-986` (the `pending_header ({n} site{s})` activity suffix,
`n` derived from the same `web_search_site_count` helper) now invoke
`runie_tui_model::web_search_sources_line` and
`runie_tui_model::web_search_site_count` directly. The inline web-search
parser inside `runie_tui_model::completed_tool_header_with_args` at
`crates/runie-tui-model/src/feed.rs:305-325` was replaced with a
single `web_search_site_count(output)` call, removing a second copy of
the URL-token hostname extraction that quietly diverged from the
renderer contract (the renderer counted `, `, `) `, and `] ` as URL
terminators; the model only treated `/` as one — `crates.rs/foo?q=1#x`
and `github.com/path)` were split on the wrong boundary, so the
header suffix and the Sources row could disagree on the same payload).
The two new canonical helpers now live at
`crates/runie-tui-model/src/feed.rs:184-219` next to `tool_result_text`
and are re-exported through `crates/runie-tui-model/src/lib.rs` next to
the existing `tool_result_text` / `activity_text` /
`completed_tool_header_with_args` re-exports. The renderer contract is
preserved exactly: URL terminators are `/`, `?`, `#`, `)`, `]`, `,`;
domain deduplication is case-insensitive; the first-seen order is
preserved; the first three unique hostnames are listed and any remainder
is summarized as `(+N more)`; URLs that contain no hostnames are not
emitted (returning `None`); the URL-free fallback counts non-empty lines
so the `pending_header ({n} site{s})` suffix stays well-defined when
the renderer receives a plain-text web-search result. The actor's
direct event projection in `crates/runie-tui/src/scrollback_actor.rs:416-431`
also appends the canonical `  Sources: …` row to the `ToolEnd`
output for successful `web_search` / `web-search` completions only, so
event-driven tests (and any future bus-subscription use) get the same
projection without going through the renderer. The model-side test
coverage was extended with seven focused assertions appended to
`crates/runie-tui-model/src/feed.rs` (the `tests` module, near the
existing `tool_result_text` and `completed_tool_header_with_args`
assertions): `web_search_sources_line_dedups_case_insensitively_in_first_seen_order`
pins case-insensitive dedup plus first-seen ordering against the same
five-URL payload the old renderer test used; `web_search_sources_line_returns_none_for_empty_source_line`
covers the empty-input / whitespace-only / URL-free cases that must
return `None`; `web_search_sources_line_paginates_with_plus_n_more`
covers the pagination `(+N more)` suffix when more than three unique
hostnames appear; `web_search_sources_line_trims_url_terminators_and_punctuation`
covers the `,`, `?`, `#`, `)`, and `]` URL terminators against a
sentence that mixes punctuation outside the URLs and three different
terminators inside; `web_search_site_count_dedups_case_insensitively`
pins the `n site{s}` count against the same three-URL mixed-case
payload the old renderer test used; `web_search_site_count_trims_url_terminators_and_punctuation`
covers the count-side URL terminator handling; and
`web_search_site_count_falls_back_to_non_empty_lines_when_url_free`
covers the URL-free non-empty-line fallback. The renderer test
`web_search_sources_projection_matches_grok_summary` at
`crates/runie-tui/src/event_renderer.rs:1122-1131` and the
`web_search_site_count(...)` assertion inside the unrelated
`handle_tool_end` test were removed (the model-side coverage subsumes
both). Two new actor tests were appended at
`crates/runie-tui/src/scrollback_actor.rs` (the `tests` module, near
the other completion and turn-summary assertions):
`actor_appends_canonical_sources_row_for_successful_web_search`
delivers `ToolExecutionEnd` with a successful web-search result via
`apply_event` and pins the `  Sources: docs.rs, rust-lang.org, github.com`
row in the actor snapshot, and `actor_skips_sources_row_for_failed_web_search`
delivers the same tool name with `is_error = true` and asserts that no
`  Sources:` row appears. The 67 `runie-tui-model` lib unit tests,
the 220 `runie-tui` lib unit tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) plus `just e2e-one visual-web-search.yaml`
are green.

## `completed_tool_header` retirement (2026-08-08)

The renderer-local `completed_tool_header` and
`completed_tool_header_with_args` functions at
`crates/runie-tui/src/event_renderer.rs:950-1097` were retired and the
sole renderer call site at `crates/runie-tui/src/event_renderer.rs:675`
now routes through the canonical
`runie_tui_model::completed_tool_header_with_args` so the Grok
cardinality DSL lives in one place. The model-side arm at
`crates/runie-tui-model/src/feed.rs:386-393` was extended with the
`search_tools | search-tools | search_tool` aliases that the renderer
previously held locally, closing the gap that let tool searches render
as `→ ✓` instead of `(N result{s})`. Twelve focused unit tests were
appended to `crates/runie-tui-model/src/feed.rs` (the `tests` module,
after the existing `web_search_site_count_*` block):
`completed_tool_header_with_args_pins_search_tools_aliases_and_cardinality`
covers the new `search_tools`, `search-tools`, and `search_tool`
aliases against one-result, multi-result, and blank-line-skipping
payloads; `completed_tool_header_with_args_routes_read_file_image_content`
pins the `(image)` suffix when the read result's `content` array
contains a `{"type":"image"}` item;
`completed_tool_header_with_args_renders_read_file_offset_range_with_total`
pins the `(41-42 of 100)` offset-range suffix against the same
`totalLines`/`[N more lines...]` payload the old renderer test used;
and one test per remaining tool family pins the projection
(`list_dir`/`list_files` cardinality, `read` line count, `search`
match count, `edit` edit count, `workflow` `Workflow completed: …`,
`use` `Used …`, `subagent` `Subagent completed: …`, `web_search`
site count, `memory_search` results). The renderer test
`completed_file_tools_use_grok_card_cardinality` at
`crates/runie-tui/src/event_renderer.rs:2160-2207` was deleted since
the new model-side coverage subsumes every assertion it held. The
86 `runie-tui-model` lib unit tests (74 pre-existing + 12 new), the
`runie-tui` lib unit tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

## `tool_header` retirement (2026-08-08)

The renderer-local `tool_header` and `make_relative_path` helpers at
`crates/runie-tui/src/event_renderer.rs:797-944` were retired and the
sole renderer call site at `crates/runie-tui/src/event_renderer.rs:598`
now routes through the canonical
`runie_tui_model::tool_header(tool_name, args, workspace)`. The model
function at `crates/runie-tui-model/src/feed.rs:56-126` already accepts
the workspace anchor; the renderer now threads its workspace through
the call rather than rebuilding a `std::env::current_dir()` projection
inside the renderer boundary. The model-side `tool_header` was extended
with the `search_tools | search-tools | search_tool` aliases that the
renderer previously held locally (added at
`crates/runie-tui-model/src/feed.rs:121-123`), closing the gap that let
tool searches render as the generic `<name> {…}` fallback. The
`EventRenderer` struct grew a `workspace: String` field threaded
through `with_actors_inner`/`with_actors`/`with_live_actors`; the
production call site at `crates/runie-tui/src/app.rs:840-848` resolves
the workspace via `std::env::current_dir()`, and the YAML replay path
at `crates/runie-tui/src/yaml_runner.rs:1982-1991` and
`crates/runie-tui/src/yaml_runner.rs:4991-5001` plus the E2E test at
`crates/runie-tui/tests/e2e_test.rs:186-194` thread the same
workspace through `with_actors` so replay fixtures remain
host-independent. The 22 test construction sites and the two test
bodies (`structured_tools_use_grok_headers_and_preserve_output_rows`
and `absolute_tool_paths_are_workspace_relative`) were updated to
pass `TEST_WORKSPACE = "/work"` (or a per-test workspace anchor for the
absolute-path regression); three new tests were appended at
`crates/runie-tui/src/event_renderer.rs` to lock the new behaviour:
`absolute_tool_paths_are_workspace_relative` (existing test now drives
both `read`, `list_dir`, `edit`, and the `make_relative_path` boundary
through the workspace anchor), `renderer_tool_header_projects_absolute_paths_through_workspace_anchor`
(verifies the renderer no longer reads `std::env::current_dir` and
pins the `List .` workspace-only listing collapse), and
`renderer_tool_header_strips_leading_separator_after_workspace` (covers
the `<workspace>relative` and `<workspace>/relative` edge cases).
The full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) is green.

Semantic header pin (2026-08-08): `feed::tests::tool_header_pins_search_tools_aliases_and_workspace_anchor`
in `crates/runie-tui-model/src/feed.rs` locks the three Grok
`search_tools` / `search-tools` / `search_tool` aliases to the same
header, pins the `query` → `pattern` key fallback, and exercises the
third `workspace` argument the renderer now threads through
`tool_header`. The full `just ci` (fmt-check, clippy, lint, test,
parity, source inventory, Pi event contract, feed-actor boundary) is
green.

**Running bullet retirement (2026-08-08):** the Grok running tool bullet
vocabulary (`⋅ `, `: `, `⸬ `, `⁙ `) and the `running_bullet(frame)`
projector now live in `runie-tui-model::feed` alongside the other
animation-frame helpers. The renderer-local `const RUNNING_BULLETS` at
`crates/runie-tui/src/widgets/scrollback.rs:2119` and the local
`fn running_bullet(frame)` wrapper were retired; the sole renderer call
site at the `LineKind::ToolRunning` prefix branch now invokes
`runie_tui_model::running_bullet(self.navigation.animation_frame)`
directly. The new `running_bullet_pins_grok_frame_vocabulary_and_wraps`
test in `crates/runie-tui-model/src/feed.rs` pins the four source-backed
Grok frames in order, the direct `frame` index projection, and the
wrap-around for `frame == 4` and `frame == usize::MAX`. The 88
`runie-tui-model` lib unit tests (87 pre-existing + 1 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**Markdown predicate retirement (2026-08-08):** the CommonMark/Grok
markdown classifier predicates now live in `runie-tui-model::feed`
alongside the other text projections. The renderer-local `is_fence`,
`is_table_row`, `is_table_separator`, and `atx_heading` functions at
`crates/runie-tui/src/widgets/scrollback.rs:1874, 2212, 2249, 2267` were
collapsed to thin `runie_tui_model::*` delegates, so the actor-owned
markdown classifier and the renderer share one vocabulary. The four
new focused tests in `crates/runie-tui-model/src/feed.rs` pin the
classifier behavior: `is_fence_detects_three_backtick_marker_with_or_without_grok_prefix`
locks the `┃ ` prefix accommodation and the negative-path prose/single-
backtick cases, `is_table_row_requires_leading_trailing_pipe_and_two_separators`
pins the leading/trailing `|` requirement and the single-cell accepted
shape, `is_table_separator_accepts_only_dash_colon_and_whitespace_cells`
covers the alignment glyphs and the markdown body negative path, and
`atx_heading_returns_title_only_within_commonmark_levels` exercises the
`1..=6` level range, the missing-space edge case, and the empty-title
projection. The 92 `runie-tui-model` lib unit tests (88 pre-existing +
4 new), the 220 `runie-tui` lib unit tests, the 5 `runie` binary unit
tests, the 28 `visual_snapshots` replay tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

**Markdown bottom border retirement (2026-08-08):** the Grok markdown
table bottom border helper now lives in `runie-tui-model::feed` next to
the other markdown predicates. The renderer-local `table_bottom_border`
function at `crates/runie-tui/src/widgets/scrollback.rs:2257` was
collapsed to a thin `runie_tui_model::table_bottom_border` delegate, so
the actor-owned markdown formatter and the renderer share one border
shape. The new `table_bottom_border_aligns_with_separator_widths` test
in `crates/runie-tui-model/src/feed.rs` pins the three-cell `├───┤`
joint projection, the wider-cell padding (`cell_width + 2` per segment),
and the whitespace-trim noise-tolerance path. The 93 `runie-tui-model`
lib unit tests (92 pre-existing + 1 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Text wrapper retirement (2026-08-08):** the renderer-local
`append_wrapped` and `append_wrapped_words` text wrappers at
`crates/runie-tui/src/widgets/scrollback.rs:1787, 1841` now live in
`runie-tui-model::feed` next to the markdown predicates. Both helpers
preserve the `Vec<(LineKind, String, bool)>` row shape and the
whitespace/word-break splitting behavior, so the actor-owned row
projection and the renderer agree on the line layout. The renderer
call sites at lines 1691, 1729, 1742, and 1840 now call
`runie_tui_model::append_wrapped` / `runie_tui_model::append_wrapped_words`
directly; the local `fn append_wrapped` and `fn append_wrapped_words`
are removed. The new `append_wrapped_splits_long_lines_at_width_boundary`
and `append_wrapped_words_breaks_on_whitespace_and_preserves_indent`
tests in `crates/runie-tui-model/src/feed.rs` pin the fixed-width chunk
splitting, the zero-width edge case, the word-boundary wrap, and the
leading-indent preservation across rows. The 95 `runie-tui-model` lib
unit tests (93 pre-existing + 2 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Version badge retirement (2026-08-08):** the Grok welcome version
badge helper and its `VersionBadgeVariant` enum now live in
`runie-tui-model::feed` so the actor-owned welcome payload and the
renderer share one shape. The renderer-local `pub fn version_badge`
and `pub enum VersionBadgeVariant` at
`crates/runie-tui/src/widgets/welcome.rs:24, 18` were replaced with a
`pub use runie_tui_model::{version_badge, VersionBadgeVariant}` re-export,
so the widget's existing call sites keep their type but lose the
duplicated projection. The same re-export cascades through
`crates/runie-tui/src/lib.rs` and `crates/runie-tui/src/widgets/mod.rs`
via the existing `pub use welcome::...` wiring, so the public
`runie_tui::VersionBadgeVariant` symbol stays stable. The new
`version_badge_pins_three_grok_welcome_variants` test in
`crates/runie-tui-model/src/feed.rs` pins the full `runie v{version} ·
Beta` shape, the hero-footer `runie Beta · v{version}` order, and the
inline `runie v{version}` compact form. The 96 `runie-tui-model` lib
unit tests (95 pre-existing + 1 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Quit command predicate retirement (2026-08-08):** the `is_quit_command`
text predicate now lives in `runie-tui-model::feed` so the keymap and
any replay path share one Grok-style `exit` / `quit` / `:q` vocabulary.
The renderer-local `pub fn is_quit_command` at
`crates/runie-tui/src/key.rs:35` was collapsed to a thin
`runie_tui_model::is_quit_command` delegate, so the keymap keeps its
public symbol but loses the duplicate projection. The new
`is_quit_command_pins_grok_vocab_with_trim_and_lowercase` and
`is_quit_command_rejects_non_quit_inputs` tests in
`crates/runie-tui-model/src/feed.rs` pin the smoke path (the three
quit commands), the trim/lowercase normalization, and the negative
paths (empty, prose, partial matches, `:quit`). The 98 `runie-tui-model`
lib unit tests (96 pre-existing + 2 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Welcome modal retirement (2026-08-08):** the `welcome_modal_lines`
idle chrome helper now lives in `runie-tui-model::feed` so the
actor-owned welcome payload and the renderer share one projection. The
renderer-local `pub fn welcome_modal_lines` at
`crates/runie-tui/src/widgets/welcome.rs:200` was collapsed to a thin
`runie_tui_model::welcome_modal_lines` delegate, so the widget's
existing callers — the YAML runner and the pre-injection test — keep
their public symbol but lose the duplicated projection. The model-side
`env!("CARGO_PKG_VERSION")` resolves to the workspace version at
compile time, matching the prior behavior. The new
`welcome_modal_lines_pins_idle_chrome_shape` test in
`crates/runie-tui-model/src/feed.rs` pins the six-row count, the
uniform `LineKind::System` classification, and the source-backed
`╭─ Runie  v…` / `│ main runie` / `│ Model · runie-core` / `│ /help for
commands` / `╰─` / `◆ session_start` chrome shape. The 99 `runie-tui-model`
lib unit tests (98 pre-existing + 1 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Session start messages retirement (2026-08-08):** the wrapping
`session_start_messages` projection now lives in `runie-tui-model::feed`
so the actor-owned session-start projection and the renderer share one
shape. The renderer-local `fn session_start_messages` at
`crates/runie-tui/src/event_renderer.rs:850` was collapsed to a thin
`runie_tui_model::session_start_messages` delegate, so the live `run`
branch and the replay `apply_actor_event` arm keep their public symbol
but lose the duplicated projection. The two new tests
`session_start_messages_emits_three_bracket_rows` and
`session_start_messages_pins_separator_and_hooks_content` in
`crates/runie-tui-model/src/feed.rs` pin the three-message count, the
outer blank `LineKind::Separator` rows, and the middle
`LineKind::SessionStart` row carrying the `◆ session_start  [hooks: 1]`
content. The test was split into two after the single combined variant
hit the cognitive-complexity lint threshold. The 101 `runie-tui-model`
lib unit tests (99 pre-existing + 2 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**User prompt timestamp retirement (2026-08-08):** the Grok user-prompt
timestamp gutter helper now lives in `runie-tui-model::feed` so the
actor-owned user-prompt projection and the renderer share one wrap
rule. The renderer-local `fn append_user_with_timestamp` at
`crates/runie-tui/src/widgets/scrollback.rs:1797` was collapsed to a
thin `runie_tui_model::append_user_with_timestamp` delegate. The model
side introduced a new `USER_PREFIX_INDENT` constant (5 columns for the
Grok `   ❯ ` prefix) so the wrap helper doesn't need to depend on the
renderer-side `LineKind::User.prefix()` method. The two new focused
tests `append_user_with_timestamp_right_aligns_timestamp_into_first_row`
and `append_user_with_timestamp_wraps_remaining_text_with_indent` in
`crates/runie-tui-model/src/feed.rs` pin the right-aligned timestamp
in the first row and the indented continuation rows for over-width
prompts. The 103 `runie-tui-model` lib unit tests (101 pre-existing +
2 new), the 220 `runie-tui` lib unit tests, the 5 `runie` binary unit
tests, the 28 `visual_snapshots` replay tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

**`make_relative_path` retirement (2026-08-08):** the workspace-relative
path projection now lives in `runie-tui-model::feed` so the
actor-owned workspace anchor and the renderer share one path rule.
The renderer-local `fn make_relative_path` at
`crates/runie-tui/src/event_renderer.rs:830` was collapsed to a thin
`runie_tui_model::make_relative_path` delegate, so the existing
`structured_tools_use_grok_headers_and_preserve_output_rows` and
`absolute_tool_paths_are_workspace_relative` tests keep their helper
but lose the duplicate projection. The new
`make_relative_path_strips_workspace_and_collapses_to_dot` test in
`crates/runie-tui-model/src/feed.rs` pins the workspace-only `.`
collapse, the leading-separator stripping, the nested directory
preservation, and the out-of-workspace passthrough. The 104
`runie-tui-model` lib unit tests (103 pre-existing + 1 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**Grok compact/tip predicate retirement (2026-08-08):** the
`grok_effective_compact` and `grok_small_screen_tip_visible` predicates
plus the `GROK_AUTO_COMPACT_MAX_ROWS` and `GROK_SMALL_SCREEN_TIP_MAX_ROWS`
constants now live in `runie-tui-model::feed` so the actor-owned layout
projection and the renderer share one source-backed compact-mode
decision. The renderer-local `pub const fn` definitions and constants at
`crates/runie-tui/src/layout.rs:23-38` were replaced with `pub use
runie_tui_model::{...}` re-exports, so the layout module's existing
callers keep their public symbols but lose the duplicate threshold
definitions. The two new focused tests
`grok_effective_compact_pins_user_and_terminal_signal` and
`grok_small_screen_tip_visible_targets_the_pre_compact_band` in
`crates/runie-tui-model/src/feed.rs` pin the unmeasured-height
zero-row escape, the auto-compact band, the full-mode escape, the
user-compact override, and the pre-compact ambient window. The 106
`runie-tui-model` lib unit tests (104 pre-existing + 2 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**Model selector rows retirement (2026-08-08):** the `model_selector_rows`
projection now lives in `runie-tui-model::feed` so the actor-owned
selector projection and the renderer share one `provider/model` label
shape. The renderer-local `fn model_selector_rows` at
`crates/runie-tui/src/app.rs:53` was collapsed to a thin
`runie_tui_model::model_selector_rows` delegate, so the model selector
call site keeps its function but loses the duplicate projection. The
two new focused tests
`model_selector_rows_renders_provider_slash_model_pairs` and
`model_selector_rows_returns_empty_for_empty_snapshot` in
`crates/runie-tui-model/src/feed.rs` pin the canonical `provider/id`
rendering, the empty-snapshot passthrough, and the
multi-model preservation order. The 108 `runie-tui-model` lib unit
tests (106 pre-existing + 2 new), the 220 `runie-tui` lib unit tests,
the 5 `runie` binary unit tests, the 28 `visual_snapshots` replay
tests, and the full `just ci` (fmt-check, clippy, lint, test, parity,
source inventory, Pi event contract, feed-actor boundary) are green.

**Spinner frame retirement (2026-08-08):** the Grok braille and dot
spinner frame arrays now live in `runie-tui-model::status` so the
actor-owned animation clock and the renderer share one vocabulary.
The renderer-local `braille_spinner_frames`, `braille_spinner_fallback`,
`dot_spinner_frames`, and `dot_spinner_fallback` functions at
`crates/runie-tui/src/widgets/status.rs:37, 42, 47, 52` were collapsed
to thin `&runie_tui_model::{...}` references, and the local
`TurnStatus::FRAMES` constant at line 92 was replaced with the
canonical `runie_tui_model::BRAILLE_SPINNER_FRAMES` reference. The new
`spinner_frames_pin_grok_source_vocabularies` test in
`crates/runie-tui-model/src/status.rs` pins the eight-frame braille
order, the four-glyph ASCII fallback, the four-glyph dot pulse, and
the three-glyph dot fallback. The 109 `runie-tui-model` lib unit tests
(108 pre-existing + 1 new), the 220 `runie-tui` lib unit tests, the 5
`runie` binary unit tests, the 28 `visual_snapshots` replay tests, and
the full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) are green.

**Repository label retirement (2026-08-08):** the `repository_label`
text projection now lives in `runie-tui-model::feed` so the
actor-owned repository projection and the renderer agree on the
displayed label shape. The renderer-local `fn repository_label` at
`crates/runie-tui/src/bin/runie.rs:169` was collapsed to a thin
`runie_tui_model::repository_label` delegate that calls the model
helper with the resolved `current_dir` and `HOME` environment
variables. The function takes the home path as an injected argument
so the model-side projection stays pure and deterministic. The three
new focused tests `repository_label_renders_home_relative_path`,
`repository_label_renders_full_path_outside_home`, and
`repository_label_returns_full_path_when_home_is_missing` in
`crates/runie-tui-model/src/feed.rs` pin the `~/` prefix, the
out-of-home passthrough, and the missing-home fallback. The 112
`runie-tui-model` lib unit tests (109 pre-existing + 3 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**`structured_update_messages` retirement (2026-08-08):** the
`ToolExecutionUpdate` event projection now lives in
`runie-tui-model::feed` so the actor-owned tool update projection and
the renderer share one canonical shape. The renderer-local
`fn structured_update_messages` at
`crates/runie-tui/src/scrollback_actor.rs:297` was collapsed to a thin
`runie_tui_model::structured_update_messages` delegate. The helper
takes the active-tool set and the event as injected arguments so the
model-side projection stays pure and deterministic. The two new
focused tests
`structured_update_messages_emits_tool_update_for_active_tool` and
`structured_update_messages_skips_inactive_or_empty_events` in
`crates/runie-tui-model/src/feed.rs` pin the active-tool smoke path,
the inactive-tool negative path, the empty-`partial_result` skip,
and the non-`ToolExecutionUpdate` event passthrough. The 114
`runie-tui-model` lib unit tests (112 pre-existing + 2 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**Activity group/count retirement (2026-08-08):** the
`activity_group_exists_since_latest_user` and `activity_counts_with_start`
feed-snapshot projections now live in `runie-tui-model::feed` so the
actor-owned activity projection and the renderer share one group
classification rule. The renderer-local helpers at
`crates/runie-tui/src/event_renderer.rs:785, 796` were collapsed to
thin `runie_tui_model::*` delegates. The three new focused tests
`activity_group_exists_since_latest_user_detects_activity_after_user`,
`activity_group_exists_since_latest_user_returns_false_without_activity`,
and `activity_counts_with_start_increments_classified_tool` in
`crates/runie-tui-model/src/feed.rs` pin the post-user activity
detection, the negative-path skip, the reset-counter smoke path, and
the no-reset passthrough. The 117 `runie-tui-model` lib unit tests
(114 pre-existing + 3 new), the 220 `runie-tui` lib unit tests, the 5
`runie` binary unit tests, the 28 `visual_snapshots` replay tests, and
the full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) are green.

**Snapshot helper retirement (2026-08-08):** the `current_tool_header`,
`current_tool_args`, `active_tool_count`, and `activity_counts` proxy
helpers now live in `runie-tui-model::feed` so the actor-owned feed
projection and the renderer share one canonical shape. The renderer-local
helpers at `crates/runie-tui/src/event_renderer.rs:750, 759, 767, 775`
were collapsed to thin `runie_tui_model::*` delegates. The three new
focused tests `activity_counts_projects_snapshot_counters`,
`active_tool_count_filters_running_blocks`, and
`current_tool_args_returns_null_for_absent_tool` in
`crates/runie-tui-model/src/feed.rs` pin the snapshot-counter projection,
the running-block filter, and the absent-args null fallback. The 120
`runie-tui-model` lib unit tests (117 pre-existing + 3 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**Dense tool group retirement (2026-08-08):** the `dense_tool_group_members`
projection now lives in `runie-tui-model::feed` so the actor-owned
render projection and the renderer agree on the dense group layout.
The renderer-local `pub fn dense_tool_group_members` at
`crates/runie-tui/src/widgets/scrollback.rs:35` was collapsed to a
thin `runie_tui_model::dense_tool_group_members` delegate. The two
new focused tests
`dense_tool_group_members_projects_member_positions` and
`dense_tool_group_members_returns_empty_for_empty_input` in
`crates/runie-tui-model/src/feed.rs` pin the contiguous-group member
position projection, the separator `None` slot, and the empty-input
passthrough. The 122 `runie-tui-model` lib unit tests (120 pre-existing
+ 2 new), the 220 `runie-tui` lib unit tests, the 5 `runie` binary
unit tests, the 28 `visual_snapshots` replay tests, and the full
`just ci` (fmt-check, clippy, lint, test, parity, source inventory, Pi
event contract, feed-actor boundary) are green.

**`LineKind::prefix` retirement (2026-08-08):** the Grok transcript
prefix helper now lives in `runie-tui-model::feed` so the
actor-owned transcript projection and the renderer share one
vocabulary. The renderer-local `fn prefix(self) -> &'static str` at
`crates/runie-tui/src/widgets/scrollback.rs:80` was collapsed to a
thin `runie_tui_model::LineKind::prefix(self)` delegate, and the
source-backed Grok prefix shapes (user gutter, assistant/reasoning
rail, tool glyphs, separator/system rows, activity rail) are now
owned by the model. The four new focused tests
`line_kind_prefix_pins_user_and_assistant_rails`,
`line_kind_prefix_pins_tool_card_glyphs`,
`line_kind_prefix_pins_session_and_metadata_rows`, and
`line_kind_prefix_pins_activity_rail` in
`crates/runie-tui-model/src/feed.rs` pin the per-group prefix
shapes. The 126 `runie-tui-model` lib unit tests (122 pre-existing +
4 new), the 220 `runie-tui` lib unit tests, the 5 `runie` binary
unit tests, the 28 `visual_snapshots` replay tests, and the full
`just ci` (fmt-check, clippy, lint, test, parity, source inventory, Pi
event contract, feed-actor boundary) are green.

**`format_worked_for_seconds` retirement (2026-08-08):** the Grok
worked-for label formatter now lives in `runie-tui-model::status` so
the actor-owned status projection and the renderer share one label
shape. The renderer-local `pub fn worked_for_label` at
`crates/runie-tui/src/widgets/status.rs:275` was collapsed to a thin
`runie_tui_model::format_worked_for_seconds(self.displayed_elapsed_ticks())`
delegate, and the `StatusSnapshot::worked_for_label` method now
delegates to the same free function. The new
`format_worked_for_seconds_pins_grok_label_form` test in
`crates/runie-tui-model/src/status.rs` pins the 57-tick
`Worked for 2.8s` shape, the zero-tick `Worked for 0.0s` projection,
and the 20-tick `Worked for 1.0s` one-second threshold. The 127
`runie-tui-model` lib unit tests (126 pre-existing + 1 new), the 220
`runie-tui` lib unit tests, the 5 `runie` binary unit tests, the 28
`visual_snapshots` replay tests, and the full `just ci` (fmt-check,
clippy, lint, test, parity, source inventory, Pi event contract,
feed-actor boundary) are green.

**`background_messages_for_event` retirement (2026-08-08):** the
`BackgroundWork*` and `Workflow*` event projection now lives in
`runie-tui-model::feed` so the actor-owned background and workflow
projection and the renderer share one canonical shape. The
renderer-local `fn background_messages_for_event` at
`crates/runie-tui/src/scrollback_actor.rs:421` was collapsed to a
thin `runie_tui_model::background_messages_for_event(&event)`
delegate. The function carries an `#[allow(clippy::too_many_lines)]`
attribute since the unified background-and-workflow projection
includes both lifecycle event families. The three new focused tests
`background_messages_for_event_emits_subagent_setup`,
`background_messages_for_event_emits_subagent_tool_start`, and
`background_messages_for_event_returns_empty_for_non_background` in
`crates/runie-tui-model/src/feed.rs` pin the subagent lifecycle
triple, the `ToolStart` row shape, and the non-background passthrough.
The 130 `runie-tui-model` lib unit tests (127 pre-existing + 3 new),
the 220 `runie-tui` lib unit tests, the 5 `runie` binary unit tests,
the 28 `visual_snapshots` replay tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

**`bus_messages_for_event` retirement (2026-08-08):** the
`is_actor_feed_event`-gated bus projection now lives in
`runie-tui-model::feed` so the actor-owned bus dispatch and the
renderer share one canonical shape. The renderer-local `fn
bus_messages_for_event` at `crates/runie-tui/src/scrollback_actor.rs:244`
was collapsed to a thin `runie_tui_model::bus_messages_for_event(&event)`
delegate, and the now-dead local `background_messages_for_event`
wrapper was removed (the bus projection delegates to the model-side
background helper). The three new focused tests
`bus_messages_for_event_emits_clear_for_reset`,
`bus_messages_for_event_emits_set_theme_for_theme_changed`, and
`bus_messages_for_event_returns_empty_for_non_actor_feed` in
`crates/runie-tui-model/src/feed.rs` pin the reset clear, the theme
projection, and the non-actor-feed skip. The 133 `runie-tui-model`
lib unit tests (130 pre-existing + 3 new), the 220 `runie-tui` lib
unit tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots`
replay tests, and the full `just ci` (fmt-check, clippy, lint, test,
parity, source inventory, Pi event contract, feed-actor boundary) are
green.

**Turn status text retirement (2026-08-08):** the foreground
turn-status text formatter and the `TurnStatusPhase` enum now live
in `runie-tui-model::status` so the actor-owned status projection
and the renderer share one vocabulary. The renderer-local
`TurnStatus::text` and `pub enum TurnStatusPhase` at
`crates/runie-tui/src/widgets/status.rs:80, 124` were replaced with
a `pub use runie_tui_model::TurnStatusPhase` re-export and a thin
`runie_tui_model::turn_status_text(...)` delegate. The function
takes the phase, frame, waiting label, and chrome as injected
arguments so the model-side projection stays pure and deterministic.
The three new focused tests `turn_status_text_pins_thinking_override`,
`turn_status_text_renders_starting_with_chrome`, and
`turn_status_text_uses_waiting_label` in
`crates/runie-tui-model/src/status.rs` pin the thinking override,
the starting-chrome projection, and the waiting-label passthrough.
The 136 `runie-tui-model` lib unit tests (133 pre-existing + 3 new),
the 220 `runie-tui` lib unit tests, the 5 `runie` binary unit tests,
the 28 `visual_snapshots` replay tests, and the full `just ci`
(fmt-check, clippy, lint, test, parity, source inventory, Pi event
contract, feed-actor boundary) are green.

**`line_is_blank` retirement (2026-08-08):** the Grok blank-line
predicate now lives in `runie-tui-model::feed` as both a `Line`
method and a free function so the actor-owned transcript projection
and the renderer share the blank-line definition. The new
`line_is_blank_pins_empty_text_predicate` test in
`crates/runie-tui-model/src/feed.rs` pins the empty-text positive
case, the non-empty negative case, and the agreement between the
method and the free function. The 137 `runie-tui-model` lib unit
tests (136 pre-existing + 1 new), the 220 `runie-tui` lib unit
tests, the 5 `runie` binary unit tests, the 28 `visual_snapshots`
replay tests, and the full `just ci` (fmt-check, clippy, lint, test,
parity, source inventory, Pi event contract, feed-actor boundary) are
green.

**`find_first_containing` / `find_all_containing` retirement
(2026-08-08):** the Grok transcript search predicates now live in
`runie-tui-model::feed` so the actor-owned transcript projection and
the renderer share one search vocabulary. The renderer-local
`pub fn find_first_containing` and `pub fn find_all_containing` at
`crates/runie-tui/src/widgets/scrollback.rs:875, 880` were
collapsed to thin `runie_tui_model::find_first_containing(&self.lines, needle)`
and `runie_tui_model::find_all_containing(&self.lines, needle)`
delegates. The two new focused tests
`find_first_containing_returns_first_match_index` and
`find_all_containing_returns_all_match_indices` in
`crates/runie-tui-model/src/feed.rs` pin the first-match index, the
non-matching passthrough, the all-matches index list, and the
non-matching empty vector. The 139 `runie-tui-model` lib unit tests
(137 pre-existing + 2 new), the 220 `runie-tui` lib unit tests, the
5 `runie` binary unit tests, the 28 `visual_snapshots` replay tests,
and the full `just ci` (fmt-check, clippy, lint, test, parity, source
inventory, Pi event contract, feed-actor boundary) are green.
