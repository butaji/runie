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
