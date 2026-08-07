# P65 — Pi model selector and scoped-model contract

Status: catalog semantics and async actor implemented; selector UI/refresh adapter open (2026-08-08)

## Source contract

Pi's model behavior is not equivalent to accepting a `provider/model` string.
`AgentSession` owns a model registry and a scoped-model list. `cycleModel()`
filters scoped models against currently available models, wraps in either
direction, preserves a scoped model's optional thinking level, and persists a
model change. The interactive model selector is a searchable component with
an `all`/`scoped` scope toggle and asynchronous model refresh.

Authoritative sources:

- `/Users/admin/Code/agents/pi/packages/coding-agent/src/core/agent-session.ts`
  (`ModelCycleResult`, `setModel`, `cycleModel`, and scoped-model accessors).
- `/Users/admin/Code/agents/pi/packages/coding-agent/src/core/model-registry.ts`.
- `/Users/admin/Code/agents/pi/packages/coding-agent/src/modes/interactive/components/model-selector.ts`.
- `/Users/admin/Code/agents/pi/packages/coding-agent/src/core/keybindings.ts`
  (`app.model.select`, forward/backward cycling).

## Current Runie boundary

Runie now also has a pure `ModelCatalog` contract with YAML-driven search and
scoped-cycle coverage, plus `ModelCatalogActor` with an owned worker, mailbox,
acknowledged commands, and a watch snapshot carrying typed catalog/selection
events. The explicit `/model provider/model` route now selects through this
actor before `LoopActor` publishes the actor-owned `ModelChanged` event, and a
YAML-tested explicit
refresh result path admits successful catalogs and preserves the prior catalog
on typed refresh failure.
`/model provider/model` route. It does not yet own a model catalog, scoped
model projection, async refresh result, selection query/index, or cycle
direction. The current route must therefore remain explicit and must not claim
selector parity.

## Required implementation

Add a pure model-catalog/state contract and an owning actor mailbox:

1. YAML declares available models, scoped models, current model, refresh result,
   query, selection index, and cycle direction.
2. A model actor owns catalog and selector state; requests are immutable and
   events (`CatalogLoaded`, `ScopeChanged`, `SelectionChanged`, `ModelChanged`,
   `RefreshFailed`) cross the mailbox/event bus.
3. `LoopActor` consumes the selected model only through the model actor's
   acknowledged result; provider execution remains unchanged until selection
   commits.
4. TUI selector rendering is a pure projection of the actor snapshot, with
   Grok palette/layout tokens and no filesystem/network work in rendering.
5. YAML replay asserts filtering, all/scoped toggling, wraparound cycling,
   unavailable scoped-model removal, failed refresh immutability, and the
   resulting prompt caption at all standard capture geometries.

Do not implement selector behavior by hardcoding a model list or by mutating
`LoopActor` state directly. Until the catalog actor and replay evidence exist,
unsupported selector commands must remain explicit capability results.
