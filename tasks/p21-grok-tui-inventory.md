# p21 — Grok TUI parity inventory

Status: active. Audited `/Users/admin/Code/agents/grok-build`, especially
`crates/codegen/xai-grok-pager` and `xai-grok-pager-render`.

The complete file-level scan is reproducible with
`scripts/source-inventory.sh` and documented in
`tasks/grok-tui-file-inventory.md`; the current source counts are 496 pager
files and 68 renderer files. This inventory remains active until every
Pi-mappable behavior has the detailed lifecycle contract recorded in
`tasks/grok-tui-behavior-inventory.md` and a matching YAML/state/full-cell
oracle.
Pi-core-mapped component has a source-backed YAML or cell-level assertion.

**Construction audit (2026-08-05):** Grok composes the agent view from a
two-column outer horizontal pad, a scrollback `HorizontalLayout` with a
one-cell accent rail, two-cell left block padding, flexible content, and a
one-cell right pad. Entries are measured before painting; display mode and
vertical padding affect height, and timestamps are overlaid on the first
content row rather than appended to message text. Runie now names and tests
these source-derived layout tokens and uses them in the chat layout.

## Theme-token parity (2026-08-05)

Opaline semantic style projections now accept the actor-selected `ThemeKind`;
status styling no longer resolves `GrokNight` unconditionally. A GrokDay
regression verifies that foreground, background, and accent cells come from
the day palette's tokens. The feed projection now receives the selected theme
through its pure render path; broader Grok state/variant coverage remains
before this inventory can be marked complete.

The selected theme is now also propagated to the feed actor on
`ThemeChanged`; `visual-theme-day.yaml` exercises the complete event-driven
path without recompilation. Feed style unit coverage verifies assistant and
activity cells resolve day-theme tokens.

**Command-palette boundary (2026-08-05):** Cast inventory exposed Grok's
`Echo Command Query Title` modal. Runie now has an actor-owned
`command_palette_open` MVU state, `Ctrl+P`/`?` actions, themed modal chrome,
and Esc close behavior. Query text, filtering, and selection movement are also
actor-owned, with a pure `CommandPaletteWidget` shared by the live binary and
YAML visual runner. `visual-command-palette.yaml` covers the filtered modal;
full Grok command registry/action execution remains open.

Typed palette actions (2026-08-06): UI activation now emits a typed
`PaletteAction` through the actor command bus. The live binary and YAML replay
match `NewSession`, `KeyboardShortcuts`, and `Quit` variants directly; the
visible labels remain a pure palette projection.

`New Session` now follows the event boundary: palette activation is reduced
by the UI actor, and the binary/YAML paths publish the existing
`AgentEvent::Reset`. `visual-command-palette-activate.yaml` covers that action;
other Grok commands remain unimplemented until their corresponding core/TUI
event contracts are added.

The palette registry is intentionally limited to the three actions currently
implemented by Runie's Pi-core-backed executable: `New Session`, `Keyboard
Shortcuts`, and `Quit`. Unsupported Grok actions are not advertised as dead
commands. Adding another entry requires its corresponding core/UI event
contract and executable consumer first.

Palette scope correction (2026-08-06): removed the previous unsupported
session-management, memory, plan, context, and model-switch entries from the
palette registry. The remaining three-action registry is declarative through
`typed_action_registry!`; generated filtering/selection helpers are shared by
the actor model and palette projection, while YAML replay remains the behavior
oracle.

## Findings

- Grok has 402 pager source files and a typed `RenderBlock` projection:
  `UserPrompt`, `AgentMessage`, `ToolCall`, `Thinking`, `System`,
  `SessionEvent`, `BgTask`, `Subagent`, `Workflow`, `Btw`, `ContextInfo`,
  `CreditLimit`, and `Stub`.
- Tool rendering is specialized for execute, read, edit, list-dir, search,
  web-search, web-fetch, memory-search, lifecycle, generic use, and other
  tools. Blocks own output, grouping, selection, display mode, accent/bullet,
  and running/finished behavior.
- `BlockContext` carries display mode (`Collapsed`, `Truncated`, `Expanded`),
  wrapping mode, selection, width, raw mode, appearance, and cwd to pure
  renderers. This is the correct reference model for replacing Runie's flat
  `LineKind` projection.
- Typed turn activity includes Thinking, Responding, ToolRunning,
  AutoCompacting, Retrying, and WaitingReason variants for model, subagent,
  task output, tasks complete, and sleep. Runie's generic waiting status is
  insufficient for parity.
- `xai-grok-pager-render` provides GrokNight, GrokDay, TokyoNight,
  RosePineMoon, OscuraMidnight, and Auto themes, with truecolor definitions and
  terminal-capability quantization. Appearance configuration covers animation,
  block backgrounds, layout, prompt, scroll, scrollbar, tool bullets, edit
  headers, todos, mermaid, and terminal behavior.
- Animation is demand-driven: the event loop schedules ticks only while a
  view reports demand. Shared ticks drive spinner frames, animated accents,
  running task bullets, thinking, workflows, and overlays. The source also has
  explicit tmux/color capability probes.
- PTY tests cover waiting labels, spinner resumption, verb-group fold/expand,
  streaming/thinking folds, prompt geometry, resize, scroll, queue/interjection
  lifecycles, cancellation, overlays, welcome variants, and terminal color
  paths. This is the authoritative feature/state matrix, not isolated row
  snapshots.

## Runie gap map and order

1. Replace ad-hoc visual flags with an actor-owned typed presentation model
   carrying block/display/waiting state.
2. Add typed waiting reasons and animation demand/frame events to the event bus.
3. Add theme/config and terminal capability projections with canonical truecolor
   comparison.
4. Model tool groups/member visibility as foldable blocks with exact gap
   geometry.
5. Add YAML event/state scenarios and full PTY captures for every state and
   animation variant; keep TestBackend as the fast deterministic path.

Implementation progress: `runie-core::types::WaitingReason` now mirrors
Grok's model/subagent/task-output/tasks-complete/sleep variants, and
`AgentEvent::Waiting` is projected by the TUI status actor. The event kind is
covered by the shared DSL and YAML event oracle. `just ci` passes after this
change.

The YAML runner now accepts `waiting: { waiting: model }` and the other typed
variants as declarative events. It omits control events from the provider
stream, then appends them to the recorded bus sequence before replay, so
scenario edits do not require recompiling the test harness.

Animation scheduling now follows the same demand boundary: the renderer owns
one restartable timer and arms it only while the status actor projects an
animated state. Idle, ready, aborted, and error states do not schedule timer
wakes. The demand predicate is covered by a unit test.

The event contract now also includes Grok's six named theme variants
(`GrokNight`, `GrokDay`, `TokyoNight`, `RosePineMoon`, `OscuraMidnight`, and
`Auto`). YAML can declare `theme: grok_day`; the status actor owns the active
projection. Full palette propagation and terminal color quantization remain
open.

Welcome theme propagation (2026-08-06): the compact welcome projection now
accepts the active actor-owned theme instead of hardwiring `GrokNight`, and the
YAML frame renderer passes that projection through. A direct `GrokDay` cell
assertion covers the accent token; the regular widget API retains its
`GrokNight` default for source-compatible unit fixtures.

Feed progress: completed directory/file tool cards now retain Grok's semantic
card headers and cardinality (`List … (entries)`, `Read … (lines)`) instead of
the generic `→ ✓` suffix. Generic tools retain their status marker. This is
covered by renderer tests and the visual tool replay.

Working-view baseline progress: the literal `Hey` → short assistant response
scenario is pinned in `visual-hey.yaml`. The full-mode layout now uses Grok's
transcript rail, assistant body alignment, and quiet branch/path
header; the fixture explicitly rejects the prior `user>`/`assistant>` feed
labels.

The same replay now verifies the turn lifecycle: the provisional `◆
Thinking…` row folds to `❙  Thought` when a non-empty assistant response
commits, while empty tool/waiting captures retain their reference chrome.

Live submitted prompts now carry a wall-clock timestamp through the user
message event and render Grok's short `h:mm AM/PM` prompt-row format. Replay
fixtures with synthetic timestamps remain deterministic; the formatter has
direct unit coverage. YAML also accepts `prompt_timestamp`, and the `Hey`
fixture pins the complete timestamped user row without recompilation.

Color parity progress: the live frame now paints the GrokNight truecolor base
(`bg #141414`, `fg #e1e1e1`) before widgets render, and feed semantics use the
Grok accent/muted/success/error palette instead of terminal-default colors.
Color-sensitive visual snapshots were refreshed and the palette projection is
unit-tested; full theme switching and terminal quantization remain open.

Live `just tui` audit also found and fixed a separate input-path issue: the
binary discarded Shift+Char events, so typing `Hey` produced `y`. The direct
terminal path now forwards shifted characters to the prompt actor; tmux
replay shows `❯ Hey` and the timestamped row before submission.

The reference feed component matrix is now recorded explicitly:

| Family | Grok variants/states | Runie coverage |
|---|---|---|
| User/assistant | prompt, markdown, tables, code, links, streaming | YAML + snapshots |
| Thinking | running spinner, collapsed `Thought`, expanded reasoning | YAML + renderer tests |
| Tool cards | execute, read, edit, list-dir, search, web, lifecycle, generic | partial; typed core events, line projection |
| Tool display | collapsed, truncated, expanded, running/finished/error | collapsed/expanded activity fixtures; block model open |
| Verb groups | Read, Listed, Searched, Ran, subagent counts; running/past verbs | activity summary projection |
| Background work | subagent, workflow, task output, waiting reasons | waiting reasons; block rendering open |
| Chrome | header meter, status telemetry, prompt/footer, doctor hint | strict feed/waiting frames |
| Effects | braille/dot spinners, animated accents, overlays, terminal capability paths | demand-driven spinner; broader effects open |

The strict fixed-grid oracle now covers both the Grok feed and waiting frames;
component/state fixtures remain the next expansion target for the partial tool
and effect families above.

Per-tool display state is now event-driven: `ToolDisplayModeChanged` carries a
tool-call ID and `Collapsed`/`Truncated`/`Expanded` mode. Tool lines retain
their originating call ID. Collapsed cards keep their semantic header while
hiding output/result rows; truncated cards keep the first output/result row.
The mixed activity and truncated activity YAML fixtures exercise these rules
independently, including a complete fixed-grid replay assertion.

The immediate task remains broader tool-card parity (specialized execute/edit/
search/web/background cards), followed by theme palette propagation and the
remaining YAML state/effect matrix. Strict feed, waiting, collapsed, mixed, and
truncated fixed-grid replays are green.

Typed tool-block projection (2026-08-06): Runie now exposes a pure
`Scrollback::tool_blocks()` projection over the actor snapshot, aligned with
Grok's block/member model for parallel IDs and display modes. This is the
foundation for the remaining foldable navigation and specialized card layout;
it does not claim those behaviors are complete.

Declarative block payload assertions (2026-08-06): YAML scenarios now pin the
ordered semantic headers and per-block output rows in addition to block count
and display mode. This verifies the actor projection's payload, while keeping
rendered truncation separate from retained state; specialized renderers and
selection/navigation remain open.

Specialized-card audit (2026-08-06): Pi-core `ToolExecutionStart.args` are
already consumed once by `tool_header` and retained through the actor-owned
line/tool-block projection. No second argument store was introduced. The next
valid parity slice is Grok's fold/member navigation and card-specific layout,
not duplicate tool metadata.

Default-mode audit (2026-08-06): Grok source defaults were checked for the
Pi-core tool families. Execute/Bash uses `Truncated` while running; Read,
Edit, ListDir, Search, Web, and Use use `Collapsed`, matching Runie's
`default_tool_display_mode`. Grok finish-mode behavior does not justify a
different Runie default; remaining work is fold/member interaction and
specialized rendering details.

Command-palette projection progress (2026-08-06): aligned the modal heading
with Grok's `Commands` vocabulary and bounded keyboard selection to the
currently filtered entry set. The state remains UI-actor-owned and the visual
YAML scenarios exercise the rendered heading without introducing a direct core
mutation path.

Command activation events (2026-08-06): palette activation now emits a typed
`UiCommand` from the UI actor after the reducer acknowledgement. The YAML
runner consumes that event to publish `Reset` for `New Session`; command
execution is no longer inferred from a mutable UI snapshot.

Web-fetch card progress (2026-08-06): Fetch completion output now uses the
Grok tool-output projection (indented neutral metadata) instead of the generic
green result-arrow row. A YAML scenario pins the URL header, HTTP metadata,
and output line kinds through the real actor replay.

Web-search card progress (2026-08-06): completion cardinality now follows
Grok's distinct-site vocabulary (`site`/`sites`) and deduplicates citation
domains before rendering the header. YAML replay covers repeated `docs.rs`
citations and a second domain.

Background-work event progress (2026-08-06): added typed core lifecycle events
for background work start/progress/finish. The scrollback projection maps them
to actor-owned semantic Subagent rows, and YAML replays the running and
completed states without inferring lifecycle from a generic tool name.

Failure-branch coverage (2026-08-06): background lifecycle wire round-trips
now pin camelCase `workId` and the YAML scenario also replays a failed worker,
ensuring the typed error state survives the core-to-feed boundary.

Source audit: Grok's `UserMessageBlock` is the only standard conversation block
that enables prompt vpad; system, session-event, thinking, background, and all
specialized tool blocks explicitly disable it. The renderer also suppresses
vpad when the available content area is smaller than three rows. Runie's
`Line::has_vpad` metadata and terminal-height-aware renderer now preserve that
distinction, with unit coverage for full, compact, and undersized panes.

Execute-card progress: shell/exec tool headers now use Grok's semantic `Run`
label with the command argument, and `visual-execute.yaml` exercises the path
through the real event replay and rendered screen assertions.

Integration discovery progress: `search_tools` now uses Grok's `Search Tools`
header and `(N results)` completion cardinality, covered by
`visual-search-tools.yaml`.

Subagent-card progress: subagent/agent/task tools now render explicit
`Subagent started:` and `Subagent completed:` lifecycle text, covered by
`visual-subagent-card.yaml`. Duration text remains intentionally deferred to
the actor-owned tool timing projection.

Theme-token audit: the compact welcome title no longer uses terminal-default
`Color::Cyan`; it now resolves through the GrokNight Opaline accent token, so
welcome chrome follows the same theme projection as the feed and status bar.

Full-screen theme propagation (2026-08-06): both live binary draw paths now
derive the terminal background from the actor-owned `StatusBar` theme
snapshot. A `ThemeChanged` event therefore updates the background as well as
feed, status, and prompt token consumers; the previous hardcoded GrokNight
background was a real palette propagation gap. The complete local gate passes.

Prompt theme projection (2026-08-06): `PromptWidget` now stores the active
theme as actor-owned view state and consumes `ThemeChanged` through
`PromptActor`. Prompt borders, cursor, placeholder, and body styles resolve
the same Opaline tokens as the rest of the screen; a day-theme render test
guards against regression.

Prompt actor event regression (2026-08-06): an async actor test now publishes
`ThemeChanged(GrokDay)`, renders the actor snapshot, and asserts the day token
at the cursor cell. This verifies the event-to-view path, not only the pure
widget reducer.

Command-palette theme propagation (2026-08-06): the palette now accepts an
explicit theme token projection, and both live draw paths pass the
actor-owned status theme. The default constructor remains GrokNight for YAML
compatibility; production overlays no longer bypass theme changes.

Live overlay theme propagation (2026-08-06): the header meter, doctor hint,
shortcut bar, and ready footer now receive the actor-owned theme explicitly
and resolve all foreground/background/muted styles through Opaline token
projections. The active-screen helper layer no longer silently falls back to
GrokNight after a theme change. `just ci` passes with 119 TUI unit tests and
the complete replay/visual suite.

Specialized tool-card progress (2026-08-06): memory search and workflow calls
now use Grok-specific semantic headers and completion vocabulary, and the YAML
runner can exercise both without recompilation. Transport-only running status
updates are treated as card state rather than transcript text, preventing
`{"status":"running"}` from leaking into the feed. The new
`visual-specialized-tools.yaml` fixture and the full-mode tool snapshot cover
the corrected event sequence and rendered output.

Status chrome theme propagation (2026-08-06): `TurnStatus` and the status
footer now resolve spinner, label, shortcut, and loading styles from the
actor-selected theme. A GrokDay regression renders both the footer and active
turn row and asserts their Opaline foreground tokens; the previous helpers
were still using terminal-default styles after a theme event. The GrokNight
default path intentionally preserves Grok's terminal-default foreground
attributes, while alternate themes use explicit Opaline tokens.

Architecture audit note: `PromptActor` and `UiActor` own mailbox/watch state,
but `Scrollback` and `StatusBar` are still shared behind `parking_lot::Mutex`
and are mutated by `EventRenderer` and the render loop. This is a remaining
SSOT/MVU migration target, not evidence of completed actor parity.

The first migration seam is now in place for status: `StatusMsg` is the
explicit reducer input, `StatusBar::apply` is the pure transition boundary,
and the reducer has regression coverage. The next step is moving that reducer
behind a mailbox/watch actor and replacing the compatibility mutex with its
read-only snapshot.

`StatusActor` now provides that mailbox/watch boundary with owned task
lifetime, acknowledged `StatusMsg` application, and a snapshot regression
test. Existing rendering still uses the compatibility path; migrating
`EventRenderer` and the live/YAML renderers to this handle is the next step.

Theme variant preservation (2026-08-06): the status actor now stores the
declared `ThemeKind` as its source of truth rather than reconstructing it from
an Opaline theme display name. TokyoNight, RosePineMoon, OscuraMidnight, and
Auto no longer silently collapse back to GrokNight when the status view is
rendered; a unit matrix covers every declared variant.

Terminal capability progress (2026-08-06): added a pure `ColorLevel` model
(`None`, `Basic`, `Ansi256`, `TrueColor`) with deterministic RGB quantization
and environment detection matching Grok's `COLORTERM`/`TERM` boundary. The
live binary quantizes its completed frame before writing to the PTY; test/YAML
buffers remain truecolor so reference casts stay deterministic. Unit tests
cover all levels and preservation of indexed/default colors.

Typed waiting parity (2026-08-06): Grok's distinct waiting subjects are now
preserved from `AgentEvent::Waiting` through the status actor into
`TurnStatus`; model, subagent, task-output, tasks-complete, and sleep labels
are no longer rendered as one generic response wait. YAML replays all typed
events in `visual-waiting-reasons.yaml`.

Background lifecycle parity (2026-08-06): typed events now model Grok's
terminal `cancelled` state and deterministic elapsed durations. YAML replay
accepts optional `elapsed_ms` on terminal events plus `background_cancel`, and
the visual fixture asserts completed, failed, and cancelled labels including
duration text and error styling.

Semantic boundary audit (2026-08-06): the four subagent lifecycle events and
actor-owned running row are now documented in `parity/tui/tool-card.md`. The
remaining gap is Grok's full foldable block/member-card navigation model; no
duplicate presentation state was introduced in the reducer.

Failure-detail parity (2026-08-06): failed background events preserve an
optional provider error and render Grok's `(error)` suffix; the YAML fixture
asserts the exact terminal row.

Actor-owned fold intent (2026-08-06): added `ScrollbackMsg::ToggleToolMode`,
which cycles a selected tool block between Grok-compatible expanded and
collapsed states while preserving truncated output as retained state. The
transition is pure reducer state owned by `ScrollbackActor`; selected-entry
navigation and the live key-to-selection wiring remain open.
The `tool_fold` YAML instruction now drives this reducer transition in replay,
and the truncated activity fixture pins the resulting expanded mode.

Live fold wiring (2026-08-06): the production `e` action now publishes the
actor-owned fold intent for the last projected tool block, falling back to the
activity-group fold only when no tool block exists. This is a deterministic
bridge until Grok-equivalent cursor navigation selects arbitrary entries.

Tool selection navigation (2026-08-06): empty-prompt Up/Down actions now
publish actor-owned previous/next selection messages over projected tool IDs;
selection wraps in transcript order and `e` folds the selected ID. Prompt
history remains unchanged when the prompt contains text. Full viewport-aware
selection boxes and non-tool block entries remain open.

Non-tool entry replay coverage (2026-08-06): `visual-activity-mixed.yaml`
now drives `entry_next`/`entry_previous` through the scrollback actor after
tool-ID selection and asserts the resulting logical selected-entry index and
cleared tool ID. This pins the reducer boundary for Grok's mixed transcript
navigation; pixel-perfect selection-box styling remains open.

Selection-surface source audit (2026-08-06): Grok's selected row uses the
semantic `bg_visual` elevated surface across the row and `selection_border` at
the edges. Runie already resolves those same theme tokens in its pure
selection projection. The remaining parity item is interaction scope: Grok's
mouse/text-selection box (including split-group behavior and optional copy/
view controls) is distinct from Runie's keyboard entry cursor and should not
be claimed as closed by the existing token test.

YAML selection oracle (2026-08-06): `tool_select: next|previous` replays
through the same scrollback actor and `selected_tool_id` asserts the resulting
projection. The mixed activity fixture now pins both forward and reverse
transcript-order selection.

Workflow phase-trail parity (2026-08-06): source inspection of Grok's
`WorkflowBlock` ingestion showed that each update retains the name/objective,
ordered phase title/state pairs, current status, elapsed duration, and active
agent count. Runie's actor-owned scrollback projection now retains and updates
that same phase trail by `run_id`; the YAML lifecycle oracle asserts the final
single-card projection. Richer per-phase glyphs and exact Grok spacing remain
an attribute-level follow-up.

Workflow card typography parity (2026-08-06): the Grok source renderer formats
terminal status before the objective (`name done in duration: objective`),
renders phase states as `✓`/`●`/`○`, and only shows active-agent count while
running. Runie's workflow formatter now follows that rule, with the YAML visual
oracle asserting the exact semantic header and full-screen occurrence.

Submitted-prompt top anchor (2026-08-06): the feed previously added Grok's
visual lead row only when no separator preceded the user line. Real submitted
events already contain a separator and vpad, so the prompt rendered one row too
high at the top. Follow-mode projection now always adds the distinct lead row;
the live timestamped submission test pins the user glyph at row 2 and the
four-size `Hey` replay remains green.

Selected-header affordance (2026-08-06): the selected tool header now swaps
Grok's collapsed bullet for the `›` fold indicator in the pure scrollback
render path; the transformation is derived from actor-owned selection and
uses existing theme tokens.

Typed card-family projection (2026-08-06): `ToolBlock` now exposes a
theme-independent `ToolCardKind` (`execute`, `read`, `edit`, `list_dir`,
`search`, web, background, or generic), and mixed/truncated YAML replays pin
the ordered family vector. This makes the next specialized renderer work an
explicit event/state contract rather than header-string matching in tests.

Web-card classification correction (2026-08-06): web-search is checked before
the generic search prefix, matching Grok's distinct `Web Search` card family.
The web-search and web-fetch YAML scenarios now assert their typed families.

Replay parity (2026-08-06): deterministic `apply_actor_event` replay now
projects the same per-tool default display mode as the live bus actor
(`Truncated` for execute/bash-like tools and `Collapsed` otherwise). Explicit
YAML `tool_mode` events still override the default through the event reducer.

Default-mode scenario (2026-08-06): `visual-tool-default-modes.yaml` verifies
that Bash and Read blocks receive `Truncated` and `Collapsed` from the event
sequence without a compiled fixture change or explicit mode instruction.

Typed tool identity (2026-08-06): actor/replay start events now publish the
tool name into the scrollback reducer separately from the display header.
`ToolBlock.kind` therefore survives completion-header rewrites and only falls
back to legacy header inference for compatibility-created rows.
Responsive layout audit (2026-08-06): Grok's source derives compact mode from
terminal height (`<= 20` rows), not merely from a boolean setting. The source
behavior is recorded here for the pending full layout migration; the current
fix is limited to the feed's absolute prompt lead so unrelated 12-row
transcript geometry remains stable.

Fold-cycle audit (2026-08-06): Grok's `Entry::toggle_fold` delegates the
transition to each block family. For the shared intermediate preview state,
the cycle is `Truncated -> Collapsed`; Runie had incorrectly promoted it to
`Expanded`. The actor reducer now returns to title-only mode, and
`visual-activity-truncated.yaml` asserts the resulting state and screen.
This keeps the behavior within Pi-core tool events while matching Grok's
visible fold semantics.

Execute completion audit (2026-08-06): Grok's `ExecuteToolCallBlock` uses a
truncated preview while interactive bash runs, then promotes it to `Expanded`
when the tool finishes so the complete result is visible. Runie now applies
that transition in the scrollback actor when the typed tool name is a shell
execute variant; the default-mode YAML replay asserts the settled mode.

Read fold audit (2026-08-06): Grok's Read block uses a three-state cycle
`Collapsed -> Truncated -> Collapsed`, unlike the two-state card families.
Runie now selects that cycle from the actor-owned typed tool name, and the
default-mode YAML scenario exercises the read fold after the completed event
sequence.

Generic running-card audit (2026-08-06): Grok's `OtherToolCallBlock` has a
running-only cycle (`Collapsed -> Truncated -> Expanded -> Truncated`) and a
different settled cycle (`Collapsed <-> Expanded`). Runie's Pi-core event
projection currently marks only the explicit background/subagent lifecycle as
`ToolRunning`; arbitrary tool execution has no typed running-state event in the
TUI reducer. This remains an explicit gap. It must be closed by carrying the
tool lifecycle's running fact through an actor-owned event before adding the
running-only fold cycle; header-string inference would produce false parity.

Ordinary tool lifecycle wiring (2026-08-06): attempts to make every ordinary
start `ToolRunning` and settle all matching IDs exposed the compatibility
pre-seed contract: YAML replay can intentionally contain a pre-seeded row plus
an actor-completed row, and those rows must not be merged by ID alone. The
change was reverted after the oracle caught header corruption. The remaining
implementation must introduce explicit row ownership/identity before closing
the ordinary running-state gap.

Completion-output boundary (2026-08-06): the same oracle also pins a separate
Grok card rule: ordinary specialized cards retain their compact invocation
header after completion, while the completed preview is emitted as a structured
output entry. A reducer row token must not be used as permission to rewrite
that header; ownership and completion formatting require independent event
contracts.

Specialized-card source audit (2026-08-06): Grok's `ListDirToolCallBlock`,
`ReadToolCallBlock`, and related blocks derive completed cardinality from the
stored output during rendering (`N entries`, `N lines`, `N matches`, and so
on). Runie's pure `completed_tool_header` formatter matches those source rules.
The remaining discrepancy is the compatibility YAML projection's placement of
the completion summary among output rows; it is covered by the mixed-activity
oracle and must be resolved together with explicit row ownership, not by
changing the formatter in isolation.

Reducer checkpoint (2026-08-06): Runie's scrollback reducer now targets the
semantic tool header when settling a card, preserving all output rows. The
mixed, truncated, and update fixtures assert this event-to-state contract;
duplicate-ID compatibility seed ownership remains a separate open item.

Ordinary running-state closure (2026-08-06): `ToolStartRunning` is now an
explicit actor-owned reducer message emitted by ordinary Pi tool lifecycle
events. Compatibility seed rows retain the legacy `ToolStart` path, so their
presentation cannot be reclassified by provider call ID. `ToolBlock.is_running`
is asserted during the event-renderer lifecycle test and through the YAML
`tool_running` projection oracle; completion settles the same opaque row.

Workflow status formatter audit (2026-08-06): compared against Grok's
`WorkflowBlockStatus` renderer and added exact tests for `failed`, `cancelled`,
and `paused` elapsed wording. Cancelled/paused statuses now use `after 1.2s`
and `at 1.2s`, while done/failed retain Grok's `in 1.2s` form; the source
distinction is no longer lost in a shared prefix helper.

Read-card truncation parity (2026-08-06): Grok's `ReadToolCallBlock` keeps
the first five and last three wrapped content lines with a standalone `…`
between them in `Truncated` mode. Runie's typed read projection now preserves
that same preview shape; other tool families retain their separate truncation
rules. A pure renderer test pins the event-owned rows without timing or
renderer-side state.

Execute-card truncation parity (2026-08-06): Grok's default execute preview
keeps the first two and last three output lines and renders the hidden count as
`… +N lines`. Runie now applies that contract to typed shell execute cards;
the pure renderer test covers the resulting preview and omitted middle.
### Fold transition closure: running generic tools

Grok's `OtherToolCallBlock::next_fold_mode` is state-dependent. While the
card is running it cycles `Collapsed -> Truncated -> Expanded -> Truncated`;
after completion it cycles `Collapsed <-> Expanded`. Runie now derives this
state from the actor-owned `ToolBlock::is_running` projection and applies the
special cycle only to running generic cards. The reducer test
`running_generic_tool_uses_grok_truncated_fold_cycle_then_settled_cycle`
replays both phases without timers or renderer state.

### Web-search sources projection (2026-08-06)

Grok's `WebSearchToolCallBlock` renders a separate `Sources:` row after the
content, preserving first-seen unique domains and adding `(+N more)` after
the first three. Runie's event projection now emits the same semantic row for
web-search results; the YAML visual replay asserts it alongside the raw URL
content, while styling remains owned by the TUI theme layer.

### Context-meter boundary audit (2026-08-06)

Grok's `context_bar.rs` resolves the denominator from the active
model/context state, while Runie's `StatusSnapshot::header_meter()` still
uses a `500K` fallback. The Pi `AgentEvent` union does not carry model
metadata, so closing this gap requires an explicit actor-owned model/context
projection event (or a core snapshot bridge), not a renderer read or a
fabricated YAML value. The missing data is recorded as a prerequisite for
exact metric parity.

Context-meter projection closure (2026-08-06): Runie status state now carries
an optional actor-owned context-window denominator. The application delivers
the active core model's window through `StatusMsg::SetContextWindow`; both the
model header projection and renderer adapter use it, retaining `500K` only
when upstream model metadata is unavailable. Unit coverage includes a 1M
meter, and no renderer reads core state directly.

The YAML replay path now covers the same delivery boundary with a declarative
`context_window` event and state/screen assertions in
`visual-context-window.yaml`. This closes the previously missing fixture-level
evidence for model-dependent Grok metrics.
