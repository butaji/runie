# UX / Interaction Test Matrix

Generated: 2026-06-18
Scope: onboarding, palette, settings, model selector, slash commands, input, tools, vim navigation, and tmux smoke tests.
Source areas audited:
- `crates/runie-tui/src/tests/` (all subdirs/files)
- `crates/runie-core/src/tests/` (login_logout, slash, form_dialog, model_selector, session_tree, etc.)
- `scripts/` (smoke-tmux.sh, ux-audit.sh, ux-edge-cases.sh)
- `AGENTS.md` (4 testing layers)

---

## 1. Existing UX Tests Summary by Feature Area

### Onboarding / Login
- **State (L1/L2)**: `crates/runie-core/src/tests/login_logout/` — provider picker, key input, validation (success/failure/retry), model selection, save/cancel navigation, empty key rejection, multiple providers, disconnect, back-stack behavior.
- **Render (L3)**: `crates/runie-tui/src/tests/onboarding_render.rs` — provider list, key field, validating panel, model toggles, empty model list, error rendering, title padding.
- **E2E (L2+L3)**: `crates/runie-tui/src/tests/onboarding_e2e.rs`, `onboarding_input.rs`, `login_flow_e2e.rs` — full flow from empty to connected model, validation hook failure, paste, backspace, space toggle, save/cancel, second provider, reset preserving model.

### Command Palette
- `crates/runie-tui/src/tests/core/palette.rs`, `crates/runie-core/src/commands/tests/palette.rs` — `/` opens palette, Ctrl+P, filtering, selection, wrapping, back-stack with sub-dialogs, message commands close palette.

### Settings Dialog
- `crates/runie-tui/src/tests/core/settings_dialog.rs` — open/close, up/down wrap, toggles for read-only, steering mode, provider, theme, vim navigation, telemetry, truncation fields exposed, all runtime-tunable config keys present.

### Model Selector
- `crates/runie-core/src/tests/model_selector.rs`, `crates/runie-core/src/commands/tests/model.rs` — filter by name/provider, case-insensitive, recent models capped/deduped, grouped rendering, select/switch, wrap, current model star.
- `crates/runie-tui/src/tests/render/render_model_selector.rs` — dialog renders groups, current star, filter prompt.

### Slash Commands (Covered)
- **Core/Session**: `help`, `quit`, `reset`, `save/load/delete/sessions`, `session`, `export/import`, `name`, `fork` (form validation), `tree` (dialog open).
- **Model**: `/model` no-args selector, provider/model args, just-model-name.
- **Safety**: `/readonly`, `/trust`, `/untrust`.
- **System**: `/theme`, `/spawn` (form + event), `/agents` (panel structure), `/skills`, `/prompt`, `/hotkeys`, `/reload`, `/copy`, `/diagnostics`.
- **Tests**: `crates/runie-core/src/tests/slash/`, `session_extra.rs`, `session_tree.rs`, `theme_slash.rs`, `safety.rs`, `subagent_cmd.rs`, `commands/tests/`.

### Input
- `crates/runie-core/src/tests/input_*.rs` — history (up/down), undo/redo, word navigation, cursor, grapheme handling, flash.
- `crates/runie-tui/src/tests/core/input.rs` — typing, backspace, submit, reset command via palette, external editor, quit commands, Ctrl+Q/ForceQuit, timer behavior.

### Tools
- `crates/runie-tui/src/tests/render/tools.rs` — tool running/done/summary rendering with spinner, checkmark, bytes, error icon, duration.
- `crates/runie-tui/src/tests/smoke.rs` — tool done renders.
- `crates/runie-core/src/tests/rapid_submit.rs` — multi-turn tool flows, queue delivery, no stuck timers.

### Vim Navigation
- `crates/runie-core/src/tests/vim_mode.rs` — mode on/off, j/k/g/G scroll, Esc enters nav, Space/i exit, y/Y copy, hints, nav at lowest element exits, Esc during active turn.
- `crates/runie-tui/src/tests/vim_mode.rs`, `render/vim_nav/` — status hints, scroll rendering, nav-mode disabled input style, accent bracket, selection wrap mapping.

### Collapse / Expand
- `crates/runie-tui/src/tests/toggle_e2e.rs`, `crates/runie-core/src/tests/core/collapse*.rs` — global toggle collapses thoughts/tools, respects global flag for new items, running tools stay expanded.

### Smoke / Integration
Smoke scripts have been removed; coverage now lives in fast Rust tests (Layer 1–3) and a small set of captured-provider replay tests.

---

## 2. Gaps: User Interactions Not Covered

High-value missing coverage:

1. **Onboarding end-to-end** — ensure the first-run flow (select provider → type key → wait for validation → select model → save) is covered by Rust L2/L3 tests.
2. **Provider management end-to-end** — disconnect/reconnect flows, provider list rendering, model selection from `/providers` after disconnect.
3. **Settings persistence & truncation** — changing truncation values, theme/provider in settings persists and renders.
4. **Model selector edge cases** — opening with zero configured providers, selecting via slash arg with unconfigured model, Ctrl+M cycling smoke.
5. **Slash commands missing end-to-end or render tests**:
   - `/new`, `/history`, `/resume`, `/share`
   - `/compact` actual execution (only form validation exists)
   - `/workflow` via slash event
   - `/steer`, `/cancel` via slash event
   - `/approve`, `/reject` (pending edits)
   - `/team`, `/solo` mode switch
   - `/copy` rendered confirmation
   - `/diagnostics` rendered output
6. **Session tree interaction** — navigation, filter cycle, selecting a branch to switch conversation.
7. **File / @ picker** — opening, filtering, selecting a file, rendering `[path]` in input.
8. **Tool UX edge cases** — tool errors render ✗, large output truncation, abort during running tool and submit new message.
9. **Queue / steering UX** — Alt+Enter follow-up, Alt+Up dequeue, queued message rendering in footer/hint.
10. **Vim nav advanced** — page up/down, search, selecting block then Enter to quote, mouse click selection.
11. **Input advanced** — Shift+Enter multiline state, bracketed paste, long input scrolling, placeholder when scrolled up.
12. **Mouse** — scroll wheel scrolls feed, click focuses input, click selects model/palette item.
13. **Resize / stress** — resize during streaming across dialogs, minimum size rendering.
14. **Trust banner** — rendering of untrusted project banner and `/trust` flow from banner state.

---

## 3. Prioritized Test Matrix

Priority: **P1** = high value / user-facing regression risk, **P2** = medium, **P3** = lower / nice to have.

### 3.1 Onboarding & Provider Management

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Onboarding | 1. Start app with no providers. 2. Select provider. 3. Type API key. 4. Wait for validation. 5. Select model. 6. Save. | Input box shows `provider/model`; dialog closes; config persisted. | L2 + L3 | `crates/runie-core/src/tests/onboarding*.rs` |
| P1 | Onboarding | Type invalid API key → validation fails → error renders → retry with valid key → save. | Error message visible; retry succeeds; model connected. | L2 + L3 | `crates/runie-tui/src/tests/onboarding_e2e.rs` |
| P1 | Provider management | Open `/providers` → disconnect active provider → dialog closes → footer shows fallback provider/model. | Active provider switches; no panic; render correct. | L2 + L3 | `crates/runie-tui/src/tests/providers_e2e.rs` |
| P2 | Provider management | Open `/providers` → Add → complete login flow → return to providers list → new provider shown. | Providers list includes new provider; active model unchanged until selected. | L2 + L3 | `crates/runie-tui/src/tests/providers_e2e.rs` |
| P2 | Onboarding | Paste a multi-line API key (bracketed paste) into key field. | Key field contains full pasted string; no accidental submit. | L2 | `crates/runie-tui/src/tests/onboarding_input.rs` |
| P2 | Onboarding | At model select, uncheck all models → press Save. | Save rejected with transient error; dialog stays open. | L2 + L3 | `crates/runie-tui/src/tests/onboarding_e2e.rs` |
| P3 | Onboarding | Cancel provider picker when no model connected → app stays on picker (cannot close). | Dialog remains open; no crash. | L2 + L3 | `crates/runie-core/src/tests/onboarding*.rs` |

### 3.2 Palette, Settings, Model Selector

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Command palette | Open palette with zero providers → type `/model` → select. | Shows message or opens login flow instead of empty selector. | L2 + L3 | `crates/runie-tui/src/tests/core/palette.rs` |
| P1 | Model selector | `/model unconfigured/model` or unknown model name. | System warning: model not available; no switch. | L2 | `crates/runie-core/src/commands/tests/model.rs` |
| P1 | Settings | Change `truncation_max_lines` and `truncation_max_bytes` via settings dialog → close → verify config values. | Config updated; dialog closes. | L2 | `crates/runie-tui/src/tests/core/settings_dialog.rs` |
| P2 | Settings | Cycle Provider or Theme in settings → render shows new value in status bar. | Status bar updates; theme applies. | L3 | `crates/runie-tui/src/tests/render/settings.rs` |
| P2 | Model selector | Ctrl+M repeatedly cycles through scoped models; render shows current model in footer. | Footer updates each cycle; wraps. | L2 + L3 | `crates/runie-tui/src/tests/model_cycle.rs` |
| P2 | Palette | In vim nav mode, press `/` → palette opens → select `/reset` → palette closes and resets. | Works from nav mode without leaving artifacts. | L2 | `crates/runie-core/src/tests/vim_mode.rs` |
| P3 | Palette | Palette shows command usage ranking after repeated commands. | Frequently used commands rank higher. | L1 | `crates/runie-core/src/commands/tests/usage.rs` |
| P3 | Settings | Open settings → resize terminal very small → settings dialog still renders without panic. | No panic; layout clipped gracefully. | L3 | `crates/runie-tui/src/tests/render/settings.rs` |

### 3.3 Slash Commands

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Session | `/new` after messages → session cleared, display name reset, provider/model kept. | New empty session; confirmation message. | L2 | `crates/runie-core/src/tests/slash/session.rs` |
| P1 | Session | `/compact 50` on session with many messages → context compacted → summary message appears. | Message count reduced; summary rendered. | L2 + L3 | `crates/runie-core/src/tests/slash/compact.rs` |
| P1 | Session | `/resume` when saved sessions exist → most recent session loaded. | Session restored; confirmation. | L2 | `crates/runie-core/src/tests/slash/session.rs` |
| P1 | Session | `/tree` → navigate up/down → select a branch → conversation switches to that branch. | Branch loaded; dialog closes. | L2 + L3 | `crates/runie-core/src/tests/session_tree.rs` |
| P1 | Subagents | `/spawn do research` → SpawnAgent event emitted; hint shows subagent hotkeys. | Event round-trips; hints update. | L2 | `crates/runie-core/src/tests/subagent_cmd.rs` |
| P1 | Subagents | `/steer agent-123 fix the bug` → SteerAgent event with correct agent/message. | Event emitted. | L2 | `crates/runie-core/src/tests/subagent_cmd.rs` |
| P1 | Mode | `/team` then `/solo` → execution mode switches; hint text reflects mode. | Mode toggles; hints correct. | L2 | `crates/runie-core/src/tests/hints.rs` |
| P2 | Edits | Agent proposes file edits → `/approve` applies them; `/reject` cancels. | Approval event emitted; reject event emitted. | L2 | `crates/runie-core/src/tests/safety.rs` |
| P2 | Workflow | `/workflow "analyze src" as analyzer` → PlanGenerated event; status shows Team mode. | Workflow parses; orchestrator state Executing. | L2 | `crates/runie-core/src/tests/slash/workflow.rs` |
| P2 | Copy | After assistant response, `/copy` → clipboard file written → confirmation rendered. | Clipboard file exists; confirmation visible. | L2 + L3 | `crates/runie-tui/src/tests/render/copy.rs` |
| P2 | Prompts | `/prompt custom` switches prompt; `/prompt` no args lists current and available. | current_prompt updated; list message rendered. | L2 + L3 | `crates/runie-core/src/commands/tests/prompts.rs` |
| P2 | Diagnostics | `/diagnostics` → diagnostics panel renders resource loading info. | Panel opens; no panic. | L3 | `crates/runie-tui/src/tests/render/diagnostics.rs` |
| P3 | History | `/history` after several submits → lists recent inputs. | System message with numbered history. | L2 | `crates/runie-core/src/tests/slash/session.rs` |
| P3 | Share | `/share` → ControlEvent::ShareSession emitted. | Event emitted. | L2 | `crates/runie-core/src/tests/slash/share.rs` |
| P3 | Agents | `/agents` → New profile → fill fields → Save → profile persisted → root list shows it. | Full CRUD round-trip. | L2 + L3 | `crates/runie-tui/src/tests/agents_manager_e2e.rs` |

### 3.4 Input & Queue

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-----------------|
| P1 | Input | Type line → Shift+Enter → type second line → Enter. | Multiline input submitted as one user message. | L2 | `crates/runie-core/src/tests/input_multiline.rs` |
| P1 | Queue | Submit while agent thinking → type follow-up → Alt+Enter → verify queued follow-up. | Follow-up queued; delivered after turn. | L2 | `crates/runie-core/src/tests/queue.rs` |
| P1 | Queue | Submit while thinking → press Alt+Up → queued message restored to input. | Dequeue restores last message. | L2 | `crates/runie-core/src/tests/queue.rs` |
| P2 | Input | Bracketed paste of long text into input. | Input contains full text; cursor at end. | L2 | `crates/runie-core/src/tests/input_paste.rs` |
| P2 | Input | Type very long input exceeding terminal width. | Input scrolls horizontally; cursor visible. | L3 | `crates/runie-tui/src/tests/render/input_scroll.rs` |
| P2 | Input | Scroll up in feed → input placeholder/hint shows scroll indicator. | Placeholder indicates scrolled-up state. | L3 | `crates/runie-tui/src/tests/render/input_placeholder.rs` |
| P3 | Input | Type emoji sequence → move cursor left/right → delete → undo. | Grapheme clusters handled correctly. | L1/L2 | `crates/runie-core/src/tests/input_grapheme.rs` |

### 3.5 Tools & Streaming

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Tools | Agent calls tool that returns error → render shows ✗ icon and error text. | Tool done with error state visible. | L3 | `crates/runie-tui/src/tests/render/tools.rs` |
| P1 | Tools | Tool returns >100KB output → output truncated in render with "…" indicator. | Large output truncated; no hang. | L3 | `crates/runie-tui/src/tests/render/tool_truncation.rs` |
| P1 | Streaming | Start streaming response → press Escape → abort → submit new message → no stuck timer. | Turn aborted; new message processes. | L2 + L3 | `crates/runie-core/src/tests/queue.rs` |
| P2 | Tools | Running tool visible → global collapse toggle → running tool still expanded; done tools collapse. | Running tool priority honored. | L3 | `crates/runie-tui/src/tests/toggle_e2e.rs` |
| P2 | Streaming | Scroll up during streaming → new content does not auto-scroll while user scrolled up. | Scroll position preserved until user scrolls down. | L2 + L3 | `crates/runie-tui/src/tests/autoscroll_render.rs` |
| P3 | Tools | Multiple tools in one turn → each renders with correct duration/order. | All tools visible; turn complete after last. | L3 | `crates/runie-tui/src/tests/render/tools.rs` |

### 3.6 Vim Navigation

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Vim nav | Enter nav mode → press `g` → top of feed; press `G` → bottom. | Scroll moves to extremes. | L2 + L3 | `crates/runie-tui/src/tests/vim_mode.rs` |
| P2 | Vim nav | In nav mode, press Ctrl+D / Ctrl+U to page down/up. | Scroll by page. | L2 | `crates/runie-core/src/tests/vim_mode.rs` |
| P2 | Vim nav | Select a block with `y` → clipboard contains block content → nav exits. | Copy event emitted; nav mode off. | L2 | `crates/runie-core/src/tests/vim_mode.rs` |
| P2 | Vim nav | Press `Y` on selected block → copies metadata (timestamp/model). | Copy metadata event emitted. | L2 | `crates/runie-core/src/tests/vim_mode.rs` |
| P3 | Vim nav | In nav mode, type `/searchterm` → highlights/search jumps. | Search UI opens. | L2 + L3 | `crates/runie-tui/src/tests/vim_mode.rs` |
| P3 | Vim nav | Mouse click on a message block while in nav mode selects it. | Selection updates; no panic. | L2 | `crates/runie-core/src/tests/vim_mode.rs` |

### 3.7 File / @ Picker

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | @ picker | Type `@` → file picker opens → navigate → Enter → `[path]` inserted in input. | Picker closes; input contains file reference. | L2 + L3 | `crates/runie-tui/src/tests/core/at_file_picker.rs` |
| P1 | @ picker | Type `@src` → picker filtered to matching files → Tab cycles → Enter. | Filter applied; selection inserts path. | L2 + L3 | `crates/runie-tui/src/tests/core/tab_file_picker_filter.rs` |
| P2 | @ picker | Esc closes picker without inserting anything. | Input unchanged; dialog closed. | L2 | `crates/runie-tui/src/tests/core/at_file_picker.rs` |

### 3.8 Smoke / Integration Regression

Smoke scripts have been removed. Coverage that previously required tmux is now implemented as Rust tests using `TestBackend` and captured-provider replays.

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Smoke | Run binary with no config → complete onboarding → submit a message. | No panic; model connected. | L2 + L3 | `crates/runie-core/src/tests/onboarding*.rs` |
| P1 | Smoke | Submit while agent streaming, then Alt+Enter follow-up, then dequeue with Alt+Up. | Queue behaves correctly. | L2 | `crates/runie-core/src/tests/queue.rs` |
| P2 | Smoke | Open `/model` → filter → select → verify footer changes. | Model switch visible; no panic. | L2 + L3 | `crates/runie-core/src/commands/tests/model.rs` |
| P2 | Smoke | Run `/save foo` → `/load foo` → `/delete foo`. | Persistence commands work end-to-end. | L2 | `crates/runie-core/src/tests/slash/session.rs` |
| P2 | Smoke | Rapidly resize terminal while streaming and while palette open. | No panic; layout recovers. | L3 | `crates/runie-tui/src/tests/render/resize*.rs` |
| P3 | Smoke | Long-running session (~50 turns) with tools → check for stuck timers / memory growth. | No panic; timers reset each turn. | L2 + L3 | `crates/runie-core/src/tests/rapid_submit.rs` |

### 3.9 Trust & Safety

| Priority | Feature Area | User Action (step-by-step) | Expected Result | Suggested Layer | Suggested File Location |
|----------|--------------|----------------------------|-----------------|-----------------|-------------------------|
| P1 | Trust | Start in untrusted project → trust banner renders → `/trust` → banner disappears → write tools enabled. | Banner gone; read_only false. | L2 + L3 | `crates/runie-tui/src/tests/render/trust_banner.rs` |
| P2 | Trust | `/untrust` while a write tool is queued/proposed → read-only enforced; approve/reject behavior. | Mode switches; no panic. | L2 | `crates/runie-core/src/tests/safety.rs` |

---

## 4. Implementation Notes

- Prefer **Layer 2 (event handling)** for command behavior and **Layer 3 (rendering)** for visual confirmation; add **Layer 4 (tmux)** only for async / integration paths that lower layers cannot catch (onboarding validation, streaming abort, resize stress, long sessions).
- Keep new tests fast — avoid real network calls or sleeps. Use validation hooks, mock providers, and captured-provider replay fixtures for provider-specific behavior.
- Avoid adding new tmux/bash smoke tests; prefer Layer 1–3 Rust tests.
- When adding new slash command coverage, place Layer 1/2 tests in `crates/runie-core/src/tests/slash/` or `commands/tests/` and Layer 3 tests in `crates/runie-tui/src/tests/render/`.
- Update `AGENTS.md` task list if implementing these as tracked tasks.

---

*File: `/Users/admin/Code/GitHub/runie-dev/docs/UX_TEST_MATRIX.md`*
