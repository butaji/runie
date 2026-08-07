# p30 — Pi-core inventory → Grok mapping → Runie plan

Status: active (2026-08-06)

## Fresh source audit (2026-08-06)

**Event-boundary audit (2026-08-06):** Pi's `AgentEvent` union in
`packages/agent/src/types.ts` contains exactly ten wire events: `agent_start`,
`agent_end`, `turn_start`, `turn_end`, `message_start`, `message_update`,
`message_end`, `tool_execution_start`, `tool_execution_update`, and
`tool_execution_end`. Runie's broader `AgentEvent` also carries local
configuration, waiting, theme, background, and workflow events, but those are
not Pi-core events. `PiAgentEvent` is the closed adapter: it accepts exactly
the ten upstream variants and rejects local-only variants. This is important
for the event-everywhere rule: local state still changes through events, but
local presentation events must never be mistaken for Pi compatibility.

The boundary is checked in two ways: the source-backed validator compares the
Rust contract with Pi's TypeScript union, and YAML replay assertions convert
each emitted event through `PiAgentEvent::try_from`. A fixture that emits a
Runie-only event in a Pi trace therefore fails instead of producing a false
parity result. The lifecycle ordering remains Pi-owned (`agent_start` before
turns, `turn_end` before the next turn or `agent_end`); viewport, theme, and
animation transitions remain separate actor-owned event streams.

**Context-tool precedence correction (2026-08-06):** `LoopActor` now keeps
non-empty caller-supplied `AgentContext.tools` in the actor snapshot and only
falls back to the registered executor tools when the context omits tools.
This matches Pi's optional `AgentContext.tools` contract and prevents the
registry from silently replacing a per-run tool set.

The distinction is now explicit in Rust as `None` versus `Some(empty)`, with
`context-state.yaml` exercising the explicit-empty path through the
no-recompile YAML runner.

Re-ran `scripts/source-inventory.sh` against the authoritative checkouts. The
current counts are Pi agent 48 files, Pi AI 174 files, Grok pager 496 files,
and Grok pager-render 68 files. The earlier 37/169 Pi counts were stale
upstream-inventory evidence; the deterministic script is authoritative. The
new Pi files must be classified before any scope decision. Green local tests
do not close the documented full-color cast, specialized-card completion-
output, or actor-owned row-identity gaps.

**Live projection follow-up (2026-08-06):** The production renderer now has an
explicit `with_live_actors` projection variant for Grok's live assistant-start
spacing. YAML replay continues through `with_actors`, so deterministic fixture
selection and event contracts remain stable. This is an intentional adapter
difference, not a second state owner: both paths consume the same event
projection and only the live layout policy changes.

## Governing scope

**Runie = pi-agent-core behavior + Grok TUI, limited to pi-core features.**

The pi repository contains more than the core package: provider catalogs,
OAuth, session/harness, skills, compaction, image APIs, and the coding-agent
application. Those are inventory inputs, not automatic Runie requirements.
The implementation target is the behavior exported by pi's agent core and the
Grok pager presentation of those behaviors.

No GitHub CI is part of this plan; verification is local through `just ci` and
runtime YAML/asciinema replay.

**Tool-alias scope audit (2026-08-06):** Grok's registry also has a `skill`
alias, but that maps to Grok's application/session `Other` block rather than
an exported Pi agent-core tool. It is intentionally excluded under the
governing “Pi core features only” scope. The Pi-relevant Grok aliases are
covered in Runie's model/header projections and replay fixtures; this
exclusion is a scope decision, not an untracked classifier gap.

## Source inventory

The source scan enumerated:

| Source | Files scanned | Functional families |
|---|---:|---|
| `pi/packages/agent/src` | 48 TypeScript files | agent loop, state, events, messages, tools, queues, stream adapters, harness boundaries |
| `pi/packages/ai/src` | 174 TypeScript files | model/usage types, provider stream contracts, API adapters, transforms, retries, auth/image boundaries |
| `grok-build/xai-grok-pager/src` | 496 Rust files | scrollback blocks, prompt, status, overlays, modals, palette, commands, layouts, session UI |
| `grok-build/xai-grok-pager-render/src` | 68 Rust files | theme tokens, terminal capabilities, colors, glyphs, wrapping, overlays, safe buffer |

The inventory is file-backed rather than inferred from screenshots. The
authoritative pi core event union is `packages/agent/src/types.ts`; the
authoritative Grok block/render contracts are under
`xai-grok-pager/src/scrollback` and `xai-grok-pager-render/src`.

**Verification refresh (2026-08-07):** `scripts/source-inventory.sh` was
rerun against both authoritative checkouts. It emitted 790 entries with the
contract counts unchanged: Pi agent 48, Pi AI 174, Grok pager 496, and
Grok pager-render 68. No upstream source delta requires a new Runie mapping in
this scan.

The complete current file listing is maintained in
`tasks/pi-core-file-inventory.md`. It is generated from both Pi source trees,
classified by capability family, and must be reviewed alongside this matrix;
file presence alone is not treated as behavioral parity evidence.

The symmetric source scan is reproducible with
`scripts/source-inventory.sh`. It emits every Pi agent/AI file and every Grok
pager/pager-render file in deterministic order, allowing the mapping review to
detect additions or removals in either upstream tree.
The expected current counts are machine-readable in
`tasks/source-inventory-contract.json`; `just source-inventory-check` validates
the contract locally and is part of `just ci`. A deliberate upstream change
must refresh the scan, classify the delta in `tasks/pi-core-file-inventory.md`,
and update that contract in the same change.
The closed Pi event union is independently checked by
`scripts/validate-pi-event-contract.py`, which compares Pi's authoritative
`packages/agent/src/types.ts` union with the generated Rust boundary. The
check is local-only and skips cleanly when the upstream checkout is absent;
with the authoritative checkout present it is part of `just ci`.

## Pi capability matrix

| Pi-core family | Representative source | Runie mapping | Status |
|---|---|---|---|
| Agent lifecycle | `agent/src/agent-loop.ts`, `agent.ts` | `LoopActor`, event bus, barriers | event order/reset covered; awaited listener settlement is tracked by p40 |
| Agent state | `agent/src/types.ts` | `AgentStateActor` + immutable snapshot | covered; workflow snapshot recently added |
| Message lifecycle | `agent/src/types.ts` | `AgentEvent`, assistant sectional events | covered by replay and TUI YAML |
| Tool lifecycle | `agent/src/agent-loop.ts` | `ToolExecutorActor`, typed tool events, and YAML `tool_execution` mode | covered; continue specialized cards |
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

Pi tool-lifecycle contract audit (2026-08-06): upstream `ToolExecutionStart`,
`ToolExecutionUpdate`, and `ToolExecutionEnd` extension events carry exactly
the fields represented by Runie's typed `AgentEvent` variants. A core wire
regression now serializes update and end events and asserts Pi's camelCase
keys (`toolCallId`, `toolName`) and absence of snake_case aliases. No missing
field was invented at the event boundary; the remaining lifecycle work is
downstream projection and YAML behavior coverage.

Agent API audit (2026-08-06): rechecked Pi `Agent` public state and control
surface against `LoopActor`, `AgentStateActor`, and the queue actors. The
Runie mapping covers model/tools/messages/system prompt/thinking level,
streaming and pending-tool projections, steering/follow-up modes and clear
operations, reset/busy/abort/wait-for-idle, awaited subscribers, and
termination-aware tool batches. These transitions enter actor mailboxes or
the typed event bus; no compatibility widget is used as core state.

The remaining inventory-only boundary is Pi's session/harness persistence
package (`agent/src/harness/session/**`, compaction, and telemetry). It is not
part of the current `runie-core` feature target; promoting it would require a
separate durable-session actor and storage contract rather than silently
claiming parity from the existing in-memory loop.

Async ownership audit (2026-08-06): every production `tokio::spawn` is owned
by a returned/stored `JoinHandle` or an enclosing actor owner. `App` returns
the renderer handle, `LoopActor` stores and awaits the current run, and
`ProviderActor` retains stream pumps in a `JoinSet` whose lifetime is owned by
the worker. YAML recorder and active-run handles are joined before replay
returns. No orphan production task was found; this remains an invariant to
re-audit whenever a new async boundary is added.

Tool usage metadata audit (2026-08-06): Pi's `AgentToolResult.usage` is part
of the serialized `ToolExecutionEnd.result`, and Runie's `AgentToolResult`,
`ToolResultMessage`, and event serialization preserve that optional field.
The TUI does not create a second tool-usage state field because that would
duplicate the core-owned source of truth; usage remains available to the
status/turn projection through the existing tool-result message boundary.

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

## `addedToolNames` semantic audit (2026-08-07)

Pi does not mutate the active `AgentTool` registry when a tool result carries
`addedToolNames`. The names are consumed later by provider adapters and
deferred-tool encoding (for example tool references in Anthropic/OpenAI
payloads). Runie preserves the field through the owned tool-result event and
message snapshots; registry mutation would be incorrect. The remaining gap is
provider-specific deferred-tool encoding, tracked with the provider boundary,
not an actor state-transfer gap.

## Session message termination metadata (2026-08-07)

Pi's JSONL message entries can carry `terminate: true` independently of the
`AgentMessage` payload. `SessionSnapshot` and `SessionActor` now preserve this
fact as actor-owned `SessionEntry::terminate` metadata and round-trip it in the
validated JSONL v4 projection. A focused regression proves the field survives
export/import; the broader Pi operation-lane records (`write_deferred`, queue,
usage, and compaction records) remain separate inventory gaps and are not
claimed by this message-lane increment.

The runtime `session-restore.yaml` fixture now restores a terminated entry and
asserts the latest actor-owned entry after the subsequent turn, keeping this
contract in the no-recompile replay path as well.

Live event delivery now carries the same fact: `SessionActor` owns a
tool-call-id → termination projection while reducing `ToolExecutionEnd`, then
attaches it to the matching `MessageEnd` session entry. The mailbox regression
proves this event sequence without sleeps or direct state mutation.

Session-lane parity correction (2026-08-07): the former inventory wording
understated the stated Runie objective. Upstream Pi also provides typed
operation lanes, durable atomic JSONL storage, torn-tail repair, forks, and
compaction context/result behavior. These are now tracked as required
implementation work in `p52-pi-session-lane-parity.md`; Runie does not claim
100% core parity until that workstream is covered by actor events and YAML
restart/recovery traces.
