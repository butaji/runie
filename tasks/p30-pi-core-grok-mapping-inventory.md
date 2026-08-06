# p30 — Pi-core inventory → Grok mapping → Runie plan

Status: active (2026-08-06)

## Fresh source audit (2026-08-06)

**Context-tool precedence correction (2026-08-06):** `LoopActor` now keeps
non-empty caller-supplied `AgentContext.tools` in the actor snapshot and only
falls back to the registered executor tools when the context omits tools.
This matches Pi's optional `AgentContext.tools` contract and prevents the
registry from silently replacing a per-run tool set.

The distinction is now explicit in Rust as `None` versus `Some(empty)`, with
`context-state.yaml` exercising the explicit-empty path through the
no-recompile YAML runner.

Re-ran `scripts/source-inventory.sh` against the authoritative checkouts. The
counts remain exact: Pi agent 37 files, Pi AI 169 files, Grok pager 496 files,
and Grok pager-render 68 files. The inventory is therefore current; green
local tests do not close the documented full-color cast, specialized-card
completion-output, or actor-owned row-identity gaps.

## Governing scope

**Runie = pi-agent-core behavior + Grok TUI, limited to pi-core features.**

The pi repository contains more than the core package: provider catalogs,
OAuth, session/harness, skills, compaction, image APIs, and the coding-agent
application. Those are inventory inputs, not automatic Runie requirements.
The implementation target is the behavior exported by pi's agent core and the
Grok pager presentation of those behaviors.

No GitHub CI is part of this plan; verification is local through `just ci` and
runtime YAML/asciinema replay.

## Source inventory

The source scan enumerated:

| Source | Files scanned | Functional families |
|---|---:|---|
| `pi/packages/agent/src` | 37 TypeScript files | agent loop, state, events, messages, tools, queues, stream adapters, harness boundaries |
| `pi/packages/ai/src` | 169 TypeScript files | model/usage types, provider stream contracts, API adapters, transforms, retries, auth/image boundaries |
| `grok-build/xai-grok-pager/src` | 496 Rust files | scrollback blocks, prompt, status, overlays, modals, palette, commands, layouts, session UI |
| `grok-build/xai-grok-pager-render/src` | 68 Rust files | theme tokens, terminal capabilities, colors, glyphs, wrapping, overlays, safe buffer |

The inventory is file-backed rather than inferred from screenshots. The
authoritative pi core event union is `packages/agent/src/types.ts`; the
authoritative Grok block/render contracts are under
`xai-grok-pager/src/scrollback` and `xai-grok-pager-render/src`.

The complete current file listing is maintained in
`tasks/pi-core-file-inventory.md`. It is generated from both Pi source trees,
classified by capability family, and must be reviewed alongside this matrix;
file presence alone is not treated as behavioral parity evidence.

The symmetric source scan is reproducible with
`scripts/source-inventory.sh`. It emits every Pi agent/AI file and every Grok
pager/pager-render file in deterministic order, allowing the mapping review to
detect additions or removals in either upstream tree.

## Pi capability matrix

| Pi-core family | Representative source | Runie mapping | Status |
|---|---|---|---|
| Agent lifecycle | `agent/src/agent-loop.ts`, `agent.ts` | `LoopActor`, event bus, barriers | covered; keep event-order YAML fixtures |
| Agent state | `agent/src/types.ts` | `AgentStateActor` + immutable snapshot | covered; workflow snapshot recently added |
| Message lifecycle | `agent/src/types.ts` | `AgentEvent`, assistant sectional events | covered by replay and TUI YAML |
| Tool lifecycle | `agent/src/agent-loop.ts` | `ToolExecutorActor`, typed tool events | covered; continue specialized cards |
| Steering/follow-up queues | agent loop/state | queue actors and mailbox DSL | covered |
| Hooks | agent loop types/driver | async hook traits and turn driver | covered by p07–p08 |
| Provider stream boundary | `agent/src/stream-fn.ts` | `StreamFn` + provider actor/replay | core boundary covered; provider catalog out of scope |
| Model/usage/cost | `ai/src/types.ts`, model helpers | Rust model/usage types and status projection | covered for core event payloads |
| Abort/cancellation | agent loop + stream options | `CancellationToken`, owned tasks | covered; retain owned-spawn lint |
| Tool argument preparation | agent tool definitions | prepare/validate/dispatch path | covered by p09–p10 |
| Tool update callback lifetime | `agent/src/types.ts` + agent loop | executor-scoped update gate | covered; late callbacks are ignored after execute settles |

The late-callback regression remains a focused Rust test rather than a YAML
fixture: YAML describes event sequences after they exist, while this contract
requires retaining and invoking a callback after the async tool promise has
settled. Encoding that with a scheduler delay would violate the event-based,
no-sleep test invariant.
| Compaction/session/harness | `agent/src/harness/**` | no core equivalent currently | explicitly out of scope; add only if promoted into runie-core |
| Provider catalog/OAuth/images | `ai/src/providers/**`, `auth/**`, `images/**` | no runie-core contract | out of scope |

## Grok mapping for pi-core features

| Pi event/state | Grok presentation source | Runie TUI projection | Required evidence |
|---|---|---|---|
| user message start/end | `scrollback/blocks/user.rs`, prompt widget | themed user card + follow anchor | four-size full-screen replay |
| assistant text sections | markdown block/render | assistant rail, markdown, wrapping | YAML section sequence + screen dump |
| thinking sections | `scrollback/blocks/thinking.rs` | actor-owned thinking fold/status | expanded/collapsed YAML |
| tool start/update/end | `scrollback/blocks/tool/**` | typed tool blocks and display modes | per-tool YAML + full-screen assertions |
| waiting reason | `views/turn_status.rs` | typed status actor state | waiting matrix YAML |
| usage/stop reason | `views/status_bar.rs`, `turn_status.rs` | status/footer projection | frozen usage replay |
| theme event | `pager-render/theme/**` | Opaline theme tokens | day/night and color-attribute oracle |
| background/workflow events | `scrollback/blocks/subagent.rs`, `workflow.rs` | actor-owned cards | lifecycle/phase-trail YAML |
| scroll/selection | `scrollback/scrollback_pane.rs` | `Scrollback` reducer + viewport | complete-screen geometry matrix |
| animation demand | pager render/event loop | owned animation frame state | deterministic frame-marker replay |

## Runie implementation order

1. Keep p01–p12 core contracts aligned with pi's exported agent behavior;
   record any deliberate boundary in `PORT_NOTES.md`.
2. Finish p16 typed block/member rendering for pi-core tool events.
3. Finish p19/p25/p28/p29 capture instrumentation so claims compare complete
   cells (glyph, width, fg, bg, attributes, coordinates) at four sizes.
4. Finish p21/p24 component inventory and source-backed docs; every mapped
   feature gets a YAML event sequence and a full-screen assertion.
5. Migrate remaining compatibility renderer paths behind actor snapshots;
   preserve pure MVU view functions and owned task handles.
6. Only introduce a macro when it removes repeated ownership/dispatch wiring
   without hiding event payloads; prefer YAML additions for behavior changes.

## Documentation audit (2026-08-06)

Updated `README.md`, `crates/runie-core/PORT_NOTES.md`, and
`crates/runie-tui/README.md` to reflect the current actor/MVU implementation,
green local gates, actual TUI feature scope, and deliberate pi package
boundaries. `AGENTS.md` remains authoritative and was not modified.

## Acceptance

- This inventory remains linked from `tasks/index.json`.
- Every “covered” row has a source-backed test or replay fixture.
- Every “out of scope” row is not used as evidence against core parity.
- No parity claim is made from a component-only snapshot when full-screen
  geometry or terminal attributes are required.
