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

Palette execution reconciliation (2026-08-08): Grok's palette is a searchable
union of shortcuts, slash commands, and skills (`docs/user-guide/03-keyboard-
shortcuts.md`, “Command Palette”). Runie intentionally advertises only the
Pi-mappable actions currently implemented by its executable: `New Session`,
`Keyboard Shortcuts`, and `Quit`. The live binary already consumes all three
through `UiActor`'s typed command broadcast; the YAML runner now consumes the
same `KeyboardShortcuts` and `Quit` variants instead of dropping them. The new
`visual-command-palette-shortcuts.yaml` fixture proves the action-to-UI-state
transition without recompilation. Grok-only slash/skill actions remain
excluded until their corresponding Pi-core contract exists.

Pi command projection update (2026-08-08): Runie now also exposes the
Pi-supported `/scoped-models` and `/session` paths. The former projects catalog
rows through `ModelCatalogActor`; the latter projects `SessionActor` stats
through a read-only `SessionInfoWidget`. Neither renderer owns or mutates the
underlying state. YAML fixtures cover both whole-screen overlays.

Complete source inventory (2026-08-08): `xai-grok-pager/src/views/modal.rs:366`
defines the source palette as these sections and entries:

- Session: New Session; New Session in Worktree; dashboard; home; delete,
  resume, share, rename, session-info, feedback.
- Context: compact history; context usage; view plan; memory.
- Model & Input: switch model; always-approve; multiline input; external
  prompt editor.
- Tools: hooks; plugins; marketplace; skills; MCP servers; manage agents.
- Other: switch theme; settings; keyboard shortcuts; how-to guides; tutorial;
  quit.

The same source applies two predicates before rendering: `/share` is removed
when sharing is disabled, and slash commands are filtered by screen-mode
support; external-editor is hidden in fullscreen. Runie's current three-entry
Pi subset is therefore intentionally not a visual claim that the Grok palette
is complete. The next eligible additions are only actions backed by existing
Pi-core events/capabilities; worktree/dashboard/home/session-management,
extensions, memory, and settings require contracts that are not currently in
`runie-core`.

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
4. Complete the typed tool/member-card model: per-member identity, fold and
   navigation semantics, and exact card geometry. Keep the existing
   actor-owned keyboard selection box separate from Grok's cell-range
   selection surface.
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

Color parity implementation is complete for the declared Runie/Grok scope:
the live frame paints the selected actor-owned theme before widgets render,
feed semantics use the Grok accent/muted/success/error palette, all declared
Grok variants preserve their identity, and `ColorLevel` quantizes RGB output
deterministically for `None`, `Basic`, `Ansi256`, and `TrueColor`. Color-
sensitive snapshots and the quantization matrix cover the projection; strict
paired full-color cast evidence remains an instrumentation concern tracked by
p19/p25, not an unimplemented renderer path.

Live `just tui` audit also found and fixed a separate input-path issue: the
binary discarded Shift+Char events, so typing `Hey` produced `y`. The direct
terminal path now forwards shifted characters to the prompt actor; tmux
replay shows `❯ Hey` and the timestamped row before submission.

The reference feed component matrix is now recorded explicitly:

| Family | Grok variants/states | Runie coverage |
|---|---|---|
| User/assistant | prompt, markdown, tables, code, links, streaming | YAML + snapshots |
| Thinking | running spinner, collapsed `Thought`, expanded reasoning | YAML + renderer tests |
| Tool cards | execute, read, edit, list-dir, search, web, lifecycle, generic | typed core events, semantic rows, range/media/error headers, and whole-screen YAML fixtures; full member-card geometry remains open |
| Tool display | collapsed, truncated, expanded, running/finished/error | actor-owned fold state, running/settled cycles, keyboard selection, and YAML fixtures; full member navigation remains open |
| Verb groups | Read, Listed, Searched, Ran, subagent counts; running/past verbs | activity summary projection |
| Background work | subagent, workflow, task output, waiting reasons | actor-owned lifecycle reducers, workflow/background YAML fixtures, and whole-screen rendering; broader Grok effect variants remain open |
| Chrome | header meter, status telemetry, prompt/footer, doctor hint | strict feed/waiting frames |
| Effects | braille/dot spinners, animated accents, overlays, terminal capability paths | actor-owned demand-driven frames with deterministic YAML ticks; terminal-native notifications/wrap capabilities are Grok shell features outside the Pi-core-limited Runie scope |

The strict fixed-grid oracle now covers both the Grok feed and waiting frames;
`visual-animation-events.yaml` covers actor cadence and demand without sleeps.
Component/state fixtures remain the next expansion target for the partial tool
and in-scope effect families above; Grok-only terminal notification/wrap
features stay explicitly out of scope unless Pi-core supplies a corresponding
event contract.

Per-tool display state is now event-driven: `ToolDisplayModeChanged` carries a
tool-call ID and `Collapsed`/`Truncated`/`Expanded` mode. Tool lines retain
their originating call ID. Collapsed cards keep their semantic header while
hiding output/result rows; truncated cards keep the first output/result row.
The mixed activity and truncated activity YAML fixtures exercise these rules
independently, including a complete fixed-grid replay assertion.

The remaining task is the cast-wide YAML state/effect matrix and exact
specialized-card geometry. Strict feed, waiting, collapsed, mixed, truncated,
selection, and background/workflow fixed-grid replays are green.

Tool-member navigation replay slice (2026-08-07): `visual-tool-selection.yaml`
now drives two `tool_select: next` events through the feed actor after three
Pi tool calls and asserts the resulting selected call ID, transcript entry,
and Grok dense-group member index. This closes the missing runtime-editable
state oracle for keyboard member navigation; mouse selection and selection-box
actions are covered separately by `visual-selection-range.yaml`.

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

Semantic paint-intent oracle (2026-08-08): the typed card-row projection now
also exposes serializable `header`, `running`, `content`, `success`, `error`,
and `muted` paint roles to YAML. `visual-tool-error.yaml` asserts the
header/error sequence through the real event → actor → model projection. This
keeps styling semantics declarative without putting terminal colors into the
model or requiring fixture recompilation.

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

Edit-card alias closure (2026-08-06): Grok's tool registry treats
`apply_patch` and `strreplace` as edit-card variants, not generic tools. The
model classifier and event-to-feed header projection now preserve that family
for exact names and argument headers, with a YAML replay and
renderer-independent regression test. Diff hunk rendering,
syntax highlighting, and foldable member navigation remain separate parity
work; this slice only closes the Pi-core tool identity boundary.

List-dir alias closure (2026-08-06): Grok maps the `ls` tool name to the
`ListDirToolCallBlock`. Runie now carries that alias through activity metrics,
semantic card classification, headers, and completion entry cardinality.
`visual-list-dir-alias.yaml` exercises the full event/replay/screen path.

Execute alias closure (2026-08-06): Grok maps `execute`,
`run_terminal_command`, and `run_terminal_cmd` to Execute cards. Runie now
preserves those aliases through default truncated mode, activity accounting,
semantic headers, and YAML replay; `visual-execute-alias.yaml` explicitly
drives the expand intent before asserting the settled card.

Search alias closure (2026-08-06): Grok maps `glob` to Search and singular
`search_tool` to Search Tools. Runie now preserves both semantic families in
the model and renderer header/card projections. `visual-search-aliases.yaml`
replays both names through supported stub tools and asserts their typed card
families and visible labels.

Alias-matrix reconciliation (2026-08-06): the source-backed comparison now
covers every Grok tool alias that maps to a Pi-core-relevant card family:
terminal execute aliases, read/edit aliases, `ls`, `glob`, `search_tool`, web
fetch/search, memory/workflow/use, and background names. The remaining
specialized-card gap is therefore not name classification; it is Grok's
foldable member geometry and interaction behavior. Mouse/text-selection UI is
outside Pi agent-core scope and must not be used to inflate the Runie feature
target.

Dense-group geometry oracle (2026-08-06): the twelve-member fixture now
asserts expanded Grok geometry directly (`Run one`, `Run eleven`, and
`Run twelve` are all visible). Collapsed hidden-prefix behavior remains
isolated in the truncated-group fixture.

Collapsed-tail geometry (2026-08-06): a dedicated twelve-member collapsed
fixture now asserts the viewport-visible tail (`Run seven` through `Run
twelve`). The source `N more` row is above the tail at the 80×30 capture
height, so the oracle intentionally checks the actual visible screen rather
than claiming a clipped row is present.

Status chrome theme propagation (2026-08-06): `TurnStatus` and the status
footer now resolve spinner, label, shortcut, and loading styles from the
actor-selected theme. A GrokDay regression renders both the footer and active
turn row and asserts their Opaline foreground tokens; the previous helpers
were still using terminal-default styles after a theme event. The GrokNight
default path intentionally preserves Grok's terminal-default foreground
attributes, while alternate themes use explicit Opaline tokens.

Architecture audit closure (2026-08-07): `PromptActor`, `UiActor`,
`ScrollbackActor`, and `StatusActor` are the production mailbox/watch owners.
`App` and `EventRenderer::with_live_actors` consume actor snapshots and do not
mutate the legacy `Scrollback`/`StatusBar` mutexes. `Projection::Legacy` and
the mutex-backed widgets remain only for synchronous compatibility/replay
tests; they are not a second production state owner. Their eventual removal is
test-harness cleanup, not an open live SSOT boundary.

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
navigation and live key-to-selection wiring are now actor-backed.
The `tool_fold` YAML instruction now drives this reducer transition in replay,
and the truncated activity fixture pins the resulting expanded mode.

Live fold wiring (2026-08-06): the production `e` action now publishes the
actor-owned fold intent for the last projected tool block, falling back to the
activity-group fold only when no tool block exists. This is a deterministic
bridge until Grok-equivalent cursor navigation selects arbitrary entries.

Tool selection navigation (2026-08-06): empty-prompt Up/Down actions now
publish actor-owned previous/next selection messages over projected tool IDs;
selection wraps in transcript order and `e` folds the selected ID. Prompt
history remains unchanged when the prompt contains text. Viewport-aware
keyboard selection boxes, non-tool block entries, and actor-owned cell-range
mouse selection are implemented.

Non-tool entry replay coverage (2026-08-06): `visual-activity-mixed.yaml`
now drives `entry_next`/`entry_previous` through the scrollback actor after
tool-ID selection and asserts the resulting logical selected-entry index and
cleared tool ID. This pins the reducer boundary for Grok's mixed transcript
navigation; the keyboard selection box is implemented and covered by the
scrollback projection tests. The YAML `visual-selection-range.yaml` fixture
now covers the separate mouse/text range model as well.

Selection-surface source audit (2026-08-06): Grok's selected row uses the
semantic `bg_visual` elevated surface across the row and `selection_border` at
the edges. Runie already resolves those same theme tokens in its pure
selection projection. The remaining parity item is interaction scope: Grok's
mouse/text-selection box (including split-group behavior and optional copy/
view controls) is distinct from Runie's keyboard entry cursor and should not
be claimed as closed by the existing token test.

### Mouse/text selection contract (2026-08-07)

The implemented event boundary is explicit. Grok's mouse drag is a
transcript-cell selection, not an entry-index selection: the input layer owns
an anchor cell and current head cell, normalizes reversed coordinates, and
projects a rectangular range across wrapped rows. A split drag may begin and
end inside different feed blocks, so the selection must be represented as
viewport-relative row/column coordinates and rehydrated after scroll/resize;
it must not be inferred from rendered text. Copy/view actions are effects
after the reducer acknowledges the selected range.

Runie now represents this contract with viewport-relative cell coordinates. The
event sequence `MouseSelectionStart -> MouseSelectionExtend* ->
MouseSelectionCommit/Clear` is reduced by the feed actor; the renderer only
paints a pure selection projection using theme tokens. YAML declares cell
coordinates and asserts the normalized range, selected text rows, and copy
intent without invoking a clipboard in tests.

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

Workflow phase paint parity (2026-08-06): the renderer now consumes the
event-reduced phase trail without reparsing lifecycle state. Phase markers are
resolved as semantic theme spans: `✓` success, `●` active, `✗` failed/error,
and `○` pending; trailing agent-count metadata remains muted. This closes the
previous all-muted phase-trail styling gap while leaving exact Grok spacing and
any richer structured phase metadata open.

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
TUI reducer. This was the initial audit state; it is superseded by the
ordinary lifecycle closure documented below. It remains here as the rejected
ID-inference approach, not as the current implementation status.

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
mixed, truncated, and update fixtures assert this event-to-state contract.
Duplicate call IDs are separated by opaque actor-owned `tool_row_id`
identity; completed rows are inactive and cannot be rewritten by a later
duplicate. The remaining authoritative gap is the full Grok typed
block/member model, not ordinary duplicate-ID lifecycle ownership.

Ordinary running-state closure (2026-08-06): `ToolStartRunning` is now an
explicit actor-owned reducer message emitted by ordinary Pi tool lifecycle
events. Compatibility seed rows retain the legacy `ToolStart` path, so their
presentation cannot be reclassified by provider call ID. `ToolBlock.is_running`
is asserted during the event-renderer lifecycle test and through the YAML
`tool_running` projection oracle; completion settles the same opaque row.

Open-item reconciliation (2026-08-06): the earlier notes describing ordinary
running-state identity, dense-group truncation, and viewport centering as open
are historical and superseded by the reducer checkpoints above. The remaining
authoritative TUI gap is the full Grok typed block/member model: per-card row
identity, fold/navigation semantics across every member, and cast-wide frame
reconciliation. Future work must target that model or a verified cast delta;
reopening the closed ID/truncation paths would regress the actor boundary.

Typed row identity increment (2026-08-10): semantic `ToolCardRow` projections
now preserve the actor-issued `Line.tool_row_id` alongside the tool-call ID and
member ordinal. The renderer's paint-intent lookup requires that opaque row
identity when available, so duplicate provider call IDs cannot borrow a
neighbor's card role merely because their text and ordinal match. Compatibility
seed rows retain the optional `None` path; focused model/renderer tests and the
full local gate cover both boundaries.

Duplicate-call member increment (2026-08-10): member ordinals now inherit the
current card header's actor row identity across continuation and status lines.
Distinct live cards with the same provider call ID therefore receive distinct
ordinals, while legacy rows without an actor identity retain call-ID grouping.
The YAML replay suite, duplicate-ID model regression, and full local gate pass.

Selection snapshot increment (2026-08-10): selected-member projections now use
the exact actor-owned selected line index rather than re-resolving its provider
call ID. Feed snapshots and compatibility snapshots therefore retain the same
duplicate-card identity used by semantic rows and paint lookup; focused model,
replay, and full CI validation pass.

Dense identity projection increment (2026-08-10): the renderer-independent feed
model now exposes an identity-aware dense-group position helper. Duplicate
provider call IDs with distinct actor row identities receive separate member
positions, while the existing call-ID helper remains compatibility-preserving.
The duplicate identity regression and complete `just ci` gate pass.

Dense renderer handoff increment (2026-08-10): physical scrollback projection
now consumes identity-aware dense positions by transcript line when deciding
which members contribute to the collapsed `N more` surface. Legacy call-ID
anchors and reveal semantics remain compatible, while duplicate live rows no
longer share hidden-member positions. Focused dense/replay tests and `just ci`
pass.

Keyboard member-selection increment (2026-08-10): feed navigation now
deduplicates selectable tool entries by the actor-derived header identity
instead of provider call ID alone. Duplicate live cards can therefore be
selected independently while compatibility-seeded rows retain their previous
grouping. The focused reducer regression, complete replay suite, and `just ci`
pass.

Typed block output ownership increment (2026-08-10): `project_tool_blocks`
now attaches continuation output to the preceding card header's actor identity
when available, rather than the latest matching provider call ID. Duplicate
live cards therefore retain separate output payloads through the model and
renderer projections; focused model/replay tests and the full CI gate pass.

Tool-cycle identity increment (2026-08-10): `SelectNextTool` and
`SelectPreviousTool` now cycle exact header line identities instead of a
call-ID-collapsed block list. Duplicate live cards can be reached independently
through the actor reducer; the reducer regression, replay suite, and full CI
gate pass.

Selected-row snapshot increment (2026-08-10): feed and compatibility snapshots
now carry the optional actor-issued selected tool row ID alongside the selected
call ID and line index. This preserves duplicate-card selection identity across
actor rehydration and renderer adapters; focused selection/replay tests and the
full CI gate pass.

Selected-row renderer increment (2026-08-10): selected-line styling and dense
output suppression now prefer the selected actor row ID, falling back to call-ID
matching only for compatibility rows. Duplicate live cards no longer style or
retain output as selected merely because their provider IDs match; focused
renderer tests and the full CI gate pass.

Selection-box identity increment (2026-08-10): the selected dense-group key
projection now narrows same-call-ID rows to the selected actor row while still
including other members of the selected group. Duplicate live cards therefore
cannot borrow one another's selection surface; focused dense-selection tests,
replay, and the full CI gate pass.

Selection predicate regression (2026-08-10): a direct renderer regression now
pins the selected-row predicate for duplicate call IDs, proving the actor row
identity chooses exactly one card while the compatibility fallback remains
available. The full workspace gate remains green.

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

Read-card metadata reconciliation (2026-08-08): the current Pi tool-result
event carries opaque result JSON, arguments, range headers, and media metadata.
Runie's actor/YAML boundary already preserves that payload and the renderer
projects the read-card fields from it. A parallel typed read-metadata event
would duplicate Pi's wire contract, so this inventory item is closed unless
the upstream event shape changes or a new Grok-only card variant is identified.
### Fold transition closure: running generic tools

Grok's `OtherToolCallBlock::next_fold_mode` is state-dependent. While the
card is running it cycles `Collapsed -> Truncated -> Expanded -> Truncated`;
after completion it cycles `Collapsed <-> Expanded`. Runie now derives this
state from the actor-owned `ToolBlock::is_running` projection and applies the
special cycle only to running generic cards. The reducer test
`running_generic_tool_uses_grok_truncated_fold_cycle_then_settled_cycle`
replays both phases without timers or renderer state.

The runtime fixture `visual-tool-running-fold.yaml` now replays the complete
running `truncated → expanded → truncated` cycle through three declarative
`tool_fold` events and asserts the final actor-owned mode and screen. This
keeps the fold contract in the no-recompile YAML path, alongside the pure
reducer oracle.

`visual-tool-settled-fold.yaml` covers the complementary settled
`expanded → collapsed → expanded → collapsed` cycle, so both state-dependent
Grok fold tables are exercised through runtime YAML.

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

Workflow phase-glyph correction (2026-08-06): source inspection of
`scrollback/blocks/workflow.rs` confirms that only `done` and `active` receive
special glyphs (`✓` and `●`); cancelled, failed, and unknown phase states all
fall back to `○`. The renderer-neutral workflow projection now follows that
rule, with a model regression covering a cancelled phase. The terminal status
itself still uses Grok's separate `◌ cancelled` label.

### Read-card range metadata audit (2026-08-06)

Grok's `acp/tracker.rs` converts completed `FileContent` metadata into
`ReadToolCallBlock::with_line_range`: the first line is `offset + 1`, and the
last line is either `offset + limit` clamped to `total_lines`, or
`total_lines` when no limit is present. The block renders this as a header
suffix, including `({start}-{end} of {total})` for a subset, and also retains
typed empty, media, and error states.

Runie carries tool-call arguments on `ToolExecutionStart`. The scrollback
actor now retains those arguments by call ID and projects the ranged header
from the completion result's `details.truncation.totalLines`, with a pure
regression for `Read src/lib.rs (41-42 of 100)`. This closes the common ranged
text-header case without making the renderer inspect core state.

The completion path still reduces generic JSON content to text for the body,
so media kind and typed read errors are not yet preserved as semantic card
metadata. Parsing formatted body text remains intentionally limited to the
fallback range calculation and must not become the long-term contract.

Required next event contract: preserve typed read metadata in the core/tool
completion projection, transfer it through a scrollback event, and reduce it
into `ToolCardRow` before rendering. YAML replay must then cover full, ranged,
empty, media, and failed reads with header and semantic-row assertions. No
range is inferred until that missing event data is preserved.

The first YAML contract slice is now implemented: tool specs may override
result `output`, `details`, and `error`, and `visual-read-range.yaml` asserts
the ranged header through the real loop and actor renderer. Remaining fixtures
for typed media and error rows stay pending until their semantic event payload
is defined.

Media closure (2026-08-06): the existing Pi-compatible
`ToolResultContent::Image` payload is now emitted by YAML `media` overrides;
the shared completion projection detects that content and renders Grok's
`Read path (image)` header. `visual-read-media.yaml` verifies the result
through the real loop, actor, and visual buffer. Error behavior remains
covered by `visual-tool-error.yaml`; a distinct typed error payload is still
not modeled because the core event already carries `is_error` and the error
text.

The YAML harness now uses `register_scenario_tool` as the single registration
source for both normal and visual replay. This keeps declarative fixture
fields and built-in tool variants consistent across all test entry points.

Current metadata boundary (2026-08-06): Re-audited the remaining typed-read
metadata item against Pi's `ToolExecutionEnd` contract. Runie's event boundary
intentionally carries the same opaque JSON result that Pi exposes; the TUI
actor owns the only projection of its `details`, content kind, and error flag.
Range and media cases are covered through the real event → actor → card path.
A future typed metadata event would duplicate the Pi payload rather than
improve parity, so remaining work is limited to any new Grok card variant
demonstrated by a Pi-carried result—not an unverified renderer-side parser.

Documentation reconciliation (2026-08-06): `ToolStartRunning`, opaque row
ownership, the running fold-cycle test, and the YAML projection are the
authoritative current contract; no generic running-card gap remains at the
reducer boundary. Earlier paragraphs retain the rejected implementation path
as audit history.

Unknown-model fallback closure (2026-08-06): The live placeholder provider
uses Pi's zero-valued unknown `Model.context_window`. The app previously
delivered that as `Some(0)`, overriding the status actor's Grok-compatible
`500K` fallback and producing `14K / 0` in captures. The model-to-status event
boundary now maps zero to `None`; known positive windows still travel through
`StatusMsg::SetContextWindow` unchanged. Existing actor and YAML context-window
assertions cover both branches.

Settled-thinking rail closure (2026-08-06): Grok's collapsed thinking block
keeps its purple collapsed accent glyph (`❙`) in the transcript gutter before
the bold `◆ Thought` summary. Runie's `TurnSummary` projection previously
used plain spaces, so the glyph was absent in every full-screen `Hey` capture.
The feed renderer now emits and theme-styles the rail only for thought
summaries; ordinary `Worked for` rows retain their neutral gutter. The
event-driven `visual-reasoning.yaml` fixture and live submission renderer test
assert the marker.

Read-error header closure (2026-08-06): Grok's `ReadToolCallBlock` keeps the
normal `Read path` header on failure and places the tool error text in a
separate red body row. Runie previously appended `✗` to the header in both
the live and bus-owned reducers. Both event paths now preserve header identity
and retain the semantic `ToolError` output row; `visual-tool-error.yaml`
asserts the source-backed shape without compiled fixture code.

Memory path display closure (2026-08-08): Grok's `MemorySearchToolCallBlock`
strips its installation-specific memory root before painting result metadata
and falls back to the final path component. The renderer-independent Runie
memory projection now applies the same `/memory/`-relative and filename
fallback rule before live and YAML consumers receive rows. The specialized
fixture and pure model tests cover both forms; styling and panel fill remain
renderer concerns.

Memory snippet surface closure (2026-08-08): Grok paints each snippet as a
panel-background row, including trailing terminal cells. Runie's live Grok
layout now emits the remaining panel cells explicitly from the theme token for
memory/search content rows. A buffer-level regression verifies the far-right
cell through the real scrollback renderer; this avoids relying on Ratatui's
implicit paragraph fill.

Pi error-vector closure (2026-08-06): `visual-tool-error.yaml` now also
asserts the complete Pi lifecycle vector through tool execution, tool-result
message boundaries, continuation turn, and `agent_end`. This keeps the
Grok-style error-card oracle coupled to core event parity rather than only
checking the final screen.
