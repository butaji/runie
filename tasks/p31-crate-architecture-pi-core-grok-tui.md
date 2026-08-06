# p31 — Crate architecture: pi core + Grok TUI

Status: in_progress (2026-08-06)

## Typed core emission (2026-08-06)

`AgentStateActor::publish_pi_event` is now the state-changing emission seam
for Pi lifecycle/message/tool events. It publishes the closed
`PiAgentEvent` contract and reduces its compatibility representation in the
same actor-owned operation. The main loop driver uses this seam for agent
start/end, turn start, and user/steering/follow-up message start/end events;
Runie-specific errors and configuration events remain on the broad
compatibility path. This is an incremental migration, not evidence that all
consumers have completed the crate split.

Progress: the first boundary extraction is complete. The renderer-independent
`ScrollState` projection now lives in `runie-tui-model`; `runie-tui` keeps a
compatibility re-export, so existing widgets and YAML replay remain stable.
The model has direct reducer tests, while the existing `visual-scroll.yaml`
continues to verify the integrated actor/rendering behavior.

The status vocabulary is now also model-owned: `Status` and `StatusMsg` live
in `runie-tui-model`, while `StatusBar` retains only terminal styling,
animation presentation, and Ratatui rendering. `runie-tui` re-exports these
types for compatibility; the status actor still remains the sole reducer
owner.

The feed vocabulary is now model-owned as well: `LineKind` and `Line` live in
`runie-tui-model`; Grok prefixes and theme styles remain renderer extensions.
The compatibility exports keep the actor and existing YAML scenarios on the
same event/reducer path.

`ScrollbackMsg` is now model-owned too. The scrollback actor accepts the
model crate's intents, while the widget retains only reducer implementation
and terminal projection. This is the complete command-vocabulary split; the
remaining feed extraction is the immutable `Scrollback` snapshot itself.

The actor now exposes `FeedSnapshot`, a renderer-independent immutable
projection containing transcript lines, viewport controls, selection, theme,
animation frame, and tool display modes. The legacy `Scrollback` snapshot is
still retained for compatibility rendering; the actor-level model snapshot
test proves the new read path is fed by the same reducer.

`App::feed_model_snapshot()` is now the application-level read seam for new
model consumers and scenario assertions. `App::scrollback_snapshot()` remains
explicitly compatibility-only until the renderer migration is complete.

The YAML runner now derives `selected_tool_id`, `selected_entry`, and
`scroll_offset` assertions from `FeedSnapshot`. Full-screen visual assertions
still intentionally use the terminal renderer path, so model and pixel
coverage remain separate.

Typed `ToolBlock` and `ToolCardKind` projections are now model-owned, and
`FeedSnapshot` carries the ordered tool-card projection. YAML tool assertions
therefore consume the same immutable model data as future renderers.

The remaining `ScenarioOutcome.tool_blocks` field is compatibility-only; all
tool-count, mode, header, output, and kind assertions read
`ScenarioOutcome.feed.tool_blocks`.

`UiMsg` is now model-owned and re-exported by `runie-tui`. `UiState` remains
in the actor layer because its reducer currently depends on command-palette
filtering; moving that reducer requires a palette vocabulary model, not a
terminal widget dependency.

The palette vocabulary is now model-owned too: `PaletteAction` is generated
by the existing typed-action DSL in `runie-tui-model`, and its filtering,
selection, and count helpers are shared by the actor reducer and widget.
`UiState` has consequently moved into the model crate without importing
terminal code.

`UiCommand` is now model-owned as the pure effect intent emitted by `UiActor`.
The runtime binary remains the effect owner: it consumes the intent and
publishes core events or performs application shutdown.

Prompt vocabulary is now model-owned as well: `InputMode` and
`PromptOutcome` are re-exported by the widget for compatibility, while key
handling, cursor geometry, history storage, and terminal rendering remain in
the prompt actor/widget.

Prompt state is now also exposed as renderer-independent `PromptSnapshot` from
`runie-tui-model`; `PromptWidget::model_snapshot` and
`PromptActor::model_snapshot` provide the actor-owned projection while the
Ratatui widget snapshot remains as a compatibility path.

Status state is now exposed as renderer-independent `StatusSnapshot`, carrying
the actor-owned state, theme, animation frame, elapsed ticks, usage, and stop
reason. `StatusActor::model_snapshot` and `App::status_model_snapshot` expose
this projection while `StatusBar` remains a compatibility renderer facade.

The model crate now also defines immutable `TuiSnapshot`, aggregating the UI,
feed, prompt, and status projections for one MVU view pass. `App::model_snapshot`
builds it from actor snapshots, and `App::view_tree` consumes that aggregate
instead of independently reading compatibility widget state.

The `runie` binary now rehydrates terminal-local `StatusBar`, `Scrollback`,
and `PromptWidget` adapters from that same immutable `TuiSnapshot` in both
initial and steady-state draw paths. Palette, header, overlay, and cursor
inputs are likewise taken from the frame snapshot, eliminating mixed-time
actor reads during one paint pass.
Focused adapter tests assert that prompt and status renderer rehydration is
lossless over the complete immutable projection, complementing the feed
adapter regression.

Theme identity remains in the core event wire for now because
`AgentEvent::ThemeChanged` is part of the compatibility contract. Extracting
it requires a serialized compatibility mapping first; no TUI-only type is
being smuggled into the core boundary during this step.

## Governing rule

Runie has exactly two product layers:

1. **Pi core** — agent behavior, state, events, queues, tools, hooks,
   cancellation, and provider stream boundary.
2. **Grok TUI** — the complete presentation reach of Grok, restricted to the
   facts and interactions exposed by pi core. Grok-only product features are
   not added to Runie.

The terminal renderer is never a source of truth. It consumes immutable view
models derived from actor snapshots.

## Declarative TUI composition contract (2026-08-06)

The TUI is organized as a browser-like pipeline:

1. **Facts** — actors own state and publish events; `runie-tui-model` exposes
   immutable snapshots.
2. **What** — `view::ViewDocument` and `Element` describe slots, overlays,
   component identity, and state ownership. They contain no terminal
   coordinates, Ratatui styles, or I/O.
3. **Measure/layout** — `layout.rs` resolves responsive regions from the
   declarative tree and viewport; it owns terminal geometry only.
4. **Resolve/paint** — appearance/theme and widgets turn semantic paint
   intents plus snapshots into terminal cells. Ratatui buffers, colors,
   modifiers, cursor placement, and VT capabilities live here.
5. **Runtime effects** — the event loop owns input, terminal writes, task
   handles, and core event publication; it never becomes a second state store.

`ViewDocument` is the explicit composition boundary: tests can assert the
“what” tree and ownership independently from “how” a backend paints it.

The live `App::render` path now consumes `FeedSnapshot` and creates a
renderer-local `Scrollback` adapter only at the terminal boundary. It no
longer reads the actor's compatibility widget snapshot as a source of truth;
the adapter round-trip is covered by a focused reducer test.

The `ScrollbackActor` watch channel now publishes `FeedSnapshot` directly.
Legacy `snapshot()` callers receive a renderer-local `Scrollback` adapter
reconstructed from that immutable model, keeping compatibility APIs available
without publishing mutable-widget state as an actor projection. The actor's
reducer remains the sole state owner and the model channel is the SSOT read
boundary.

`StatusActor` follows the same boundary: its watch channel publishes
`StatusSnapshot`, while legacy `snapshot()` callers receive a local
`StatusBar` adapter. No actor watch channel now publishes the mutable TUI
widget types used only for terminal rendering.

`PromptActor` now publishes `PromptSnapshot` through its watch channel as
well. Its legacy `snapshot()` method reconstructs a `PromptWidget` only for
compatibility rendering; prompt state, cursor geometry, history, and theme
remain actor-owned model facts.

The `UiActor` audit is already closed: its watch channel has always carried
model-owned `UiState` from `runie-tui-model`, so no compatibility widget
migration is required there. A source scan confirms no TUI actor publishes
`Scrollback`, `StatusBar`, or `PromptWidget` through a watch channel.

## Target workspace

```text
crates/
├── runie-core/                 # pi-agent-core port; no terminal dependencies
│   └── src/
│       ├── types/               # messages, models, tools, usage, errors
│       ├── events/              # facts, bus, subscriptions, event DSL
│       ├── state/               # AgentStateActor + immutable snapshots
│       ├── loop/                # turn driver and LoopActor
│       ├── queues/              # steering/follow-up actor-owned queues
│       ├── tools/               # registry, validation, execution actor
│       ├── provider/            # StreamFn, provider actor, replay adapter
│       └── hooks/               # turn/tool hook contracts
│
├── runie-tui-model/             # core-event → TUI facts and view models
│   └── src/
│       ├── actors/               # Status, Scrollback, Prompt, Ui actors
│       ├── messages/             # TUI intents and reducer messages
│       ├── projection/           # pure event reducers and snapshots
│       ├── view_model/           # immutable Grok-shaped component data
│       └── scenarios/             # YAML scenario schema/loader
│
├── runie-tui-theme/             # Opaline/Grok semantic tokens only
│   └── src/
│       ├── tokens.rs              # roles, not widget-specific colors
│       ├── palettes.rs            # Grok day/night + terminal-native
│       └── resolve.rs              # token → terminal style
│
├── runie-tui-render/             # pure view model → terminal cells
│   └── src/
│       ├── layout/                # responsive geometry
│       ├── components/            # feed, prompt, status, tool, workflow
│       ├── animation/             # deterministic frame selection
│       └── cell.rs                # glyph/fg/bg/attributes output
│
├── runie-tui/                     # runtime shell and executable
│   └── src/
│       ├── runtime.rs             # terminal/event loop only
│       ├── wiring.rs              # actor/bus composition
│       └── bin/runie.rs           # CLI entry point
│
└── runie-parity/                 # dev/test-only capture and comparison
    └── src/
        ├── yaml.rs                # event sequence scenarios
        ├── capture.rs             # tmux/asciinema/VT capture adapters
        ├── oracle.rs              # complete-cell comparisons
        └── matrix.rs              # standard terminal sizes
```

## Dependency direction

```text
runie-core
    ▲
    │ facts/intents
runie-tui-model ──► runie-tui-theme
    │ view models
    ▼
runie-tui-render ──► ratatui/crossterm/VT
    ▲
runie-tui ──────── runtime wiring only

runie-parity ─────► core + tui-model + tui-render (dev/test only)
```

Rules:

- `runie-core` must not depend on any TUI crate.
- `runie-tui-model` may depend on `runie-core`, never on ratatui or crossterm.
- `runie-tui-theme` may depend on Opaline, never on actors or terminal I/O.
- `runie-tui-render` may depend on model/theme and terminal cell libraries,
  but cannot publish core events or mutate actor state.
- `runie-tui` owns terminal setup, input decoding, shutdown, and task handles.
- `runie-parity` is not a runtime dependency and cannot define production
  behavior.

## Migration order

1. Extract the remaining model contracts from `event_renderer.rs`,
   `status_actor.rs`, `scrollback_actor.rs`, and prompt/UI actor modules.
2. Extract theme tokens and palette resolution from `appearance.rs` and
   `terminal_color.rs` into `runie-tui-theme`.
3. Move pure widgets, layout, and cell snapshots into `runie-tui-render`.
4. Reduce `runie-tui` to runtime wiring and binaries.
5. Move YAML/asciinema comparison helpers into `runie-parity`; retain tests
   that exercise the public crate boundaries.
6. Delete compatibility paths only after actor replay and four-size visual
   tests pass through the new boundaries.

## Acceptance

- A compile-time dependency check prevents core → TUI edges.
- Every TUI state transition is an event reduced by one owning actor.
- Rendering accepts only immutable view models and emits complete terminal
  cells (glyph, width, foreground, background, and attributes).
- The pi-core feature inventory in `tasks/p30-*` maps to at least one model
  projection and one YAML/replay test.
- Grok-only features are explicitly excluded rather than silently stubbed.
- `just ci` remains the required local gate throughout migration.

## Boundary audit (2026-08-06)

The current `runie-core::AgentEvent` still contains Pi-independent variants
(`ThemeChanged`, `ToolDisplayModeChanged`, background-work, workflow, waiting,
reset, and UI error events) alongside the Pi lifecycle contract. They are
actor-reduced and tested, but this is not yet a strict “Pi Core only” boundary.
The migration must introduce a typed TUI/application event envelope (or an
explicit adapter) so Pi-compatible core events remain closed and Grok/TUI
projection events cannot become core feature surface. This is an architectural
gap, not a reason to weaken current replay coverage.

The first migration increment is now present as `runie_core::PiAgentEvent`.
It mirrors Pi's closed lifecycle/tool event set, converts back to the existing
bus representation, and rejects application-only variants at the boundary.
The runtime bus still uses `AgentEvent` for compatibility; migrating actors
and providers to the closed type is the next step.

The event bus now exposes `publish_pi` and an async `subscribe_pi` facade.
The facade filters application-only events while retaining the existing
compatibility receiver, providing an incremental migration path for actors
without weakening SSOT ownership or event-based delivery.

Core now exposes the same boundary through `EventBus::publish_pi` and
`subscribe_pi`; the latter asynchronously filters legacy application events.
This is the first production-facing typed bus path. Existing subscriber
registries remain compatibility adapters until their callback contract is
migrated to `PiAgentEvent`.

`LoopActor::subscribe_pi` now exposes the typed path at the actor boundary,
keeping subscription ownership and async delivery inside the actor runtime.

`SubscriberRegistry::register_pi` now provides that adapter path for async
Pi-only consumers. Its regression sequence proves application events are
ignored while Pi lifecycle events preserve ordered delivery.

The Pi boundary declaration and both conversions are now generated by the
`pi_event_contract!` macro. Its declarative field-renaming support preserves
Pi's `assistantMessageEvent` wire key, with a serialization regression test.
