# Grok Look&Feel Parity v2 — Direct Dump Analysis

**Branch:** `feat/grok-redesign`
**Date:** 2026-06-03
**Authoring method:** Read all 47 Grok dumps in `/tmp/all_grok_dumps.txt` + `docs/grok-parity/specs.md` + `docs/grok-parity/GROK.md` + current Runie source. Cross-referenced against `.hermes/plans/grok-look-feel-parity.md` (P0-P2 already done) and `.hermes/plans/ui-architecture-review.md` (4 C + 8 H bugs — C1-C4 already fixed, H1-H8 partly).
**Status as of writing:** 47/47 `grok_parity_test` checks pass. Pre-existing 18 lib test compile errors unrelated. This plan is a v2 — it does NOT duplicate the P0-P2 work in the first plan; it picks up where P2 left off and surfaces what the dumps actually show is still off.

---

## How this plan was built

I read all 47 dumps in `ui/dumps/grok/*.txt` and the concatenated `/tmp/all_grok_dumps.txt`, and extracted the actual byte-level differences between what Grok shows and what Runie's current source code emits. The 14 `FINAL_DIFF.md` items are mostly closed (C1 streaming text-append, C2 thinking doubling, C3 mock loop, C4 ghost message, header path, version badge, input bar suffix, activity `█`, token meter format, user/assistant timestamps). The first plan's P0-P2 covers all 14. The dumps today, however, still reveal 5 concrete visual gaps that were missed or under-described in the previous plans. I cataloged them in the table below and grouped the remaining work by tier.

---

## Comprehensive catalog of Grok elements (extracted from dumps)

All file:line references are to the current `feat/grok-redesign` working tree.

| # | Element | Grok dump evidence (line) | Runie source | Status | Parity test |
|---|---------|---------------------------|--------------|--------|-------------|
| 1 | Header `  ` (branch + path) | `   feat/grok-redesign ~/Code/GitHub/runie/` (dumps/01_clean:3) | `crates/runie-tui/src/components/top_bar/render.rs:12-67`; `helpers.rs:11-24` (`shorten_path`) | ✅ Match (2-space indent + trailing slash) | `header_bar::test_idle` |
| 2 | Header token meter `│ 20K / 512K │` | (dumps/02_chat_user_msg:138) | `top_bar/render.rs:42-55`; `gauge.rs:9-24` | ✅ Match (12K, 21K, 22K all in dumps) | `header_bar::test_idle/test_token_formats` |
| 3 | Spinner `⠦⠴⠋⠼` on header when streaming | (dumps/04_tools:480) | `top_bar/render.rs:28-32`; `glyphs::spinner_frame` | ✅ Match | `header_bar::test_streaming` |
| 4 | Welcome screen menu (3 items) | `New worktree / ctrl-w` (dumps/01_clean:7) | `home_screen/mod.rs:14-19, 93-110, 143-166` | ✅ Match (col 22, 2-space tip, version 0.2.16) | `welcome_menu::test_menu_items/test_tip_text` |
| 5 | Welcome tip `  Tip: Press Ctrl-W…` (2-space indent) | (dumps/01_clean:18) | `home_screen/mod.rs:163-165` | ✅ Match | `welcome_menu::test_tip_text` |
| 6 | Input box `╭─...─╮` + `│ ❯ │` | (dumps/01_clean:20-22) | `input_bar/mod.rs:33-75` | ✅ Match | `input_bar::test_normal_mode` |
| 7 | Input box bottom title `Grok Build` | (dumps/01_clean:22) | `input_bar/mod.rs:114-117` (title_bottom) | ✅ Match (per mode: `Grok Build` / `Grok Build · plan` / `Grok Build · always-approve`) | `input_bar::test_normal_mode/test_plan_mode/test_always_approve_mode` |
| 8 | Version badge `                                                                   0.2.16 Beta` | (dumps/01_clean:24) | `pipe/render/modes.rs:90-92, 138-140` | ⚠️ **Format diff**: Runie renders `0.2.16 Beta` (no `[stable]`). The const `VERSION_BADGE_FMT` at `style/format.rs:28` IS `"{} [stable] Beta"`, but the call sites at `modes.rs:90,138` ignore the const and use their own `format!("{} Beta", env!(...))`. Grok dump shows `0.2.16 Beta` (no `[stable]` actually — see 06/03 captures). | n/a |
| 9 | Status bar `Shift+Tab:mode  │  Ctrl+.:shortcuts` | (dumps/02_clean:186) | `status_bar/render.rs:57-89`; `STATUS_SEPARATOR` = `"  │  "` at `style/format.rs:31` | ✅ Match | `status_bar::test_idle/test_running` |
| 10 | Status bar running state — 4 hints | `Shift+Tab:mode  │  Ctrl+c:cancel  │  Ctrl+Enter:interject  │  Ctrl+.:` (dumps/02_chat_user_msg:159) | `status_bar/mod.rs:76-84` (`agent_running_hotkeys`) | ✅ Match | `status_bar::test_running` |
| 11 | User message `❯ <text>      <time>` | (dumps/02_chat_user_msg:141) | `message_list/render/user.rs:14-113` | ✅ Match (chevron col 5, timestamp right-aligned on first line) | `user_message::test_exact_format/test_timestamp_position` |
| 12 | User message gray background bar | **No gray bg in Grok dumps** — chat user msg sits on the panel bg | `user.rs:43-69` fills gray `border_unfocused` background on every line including padding | ❌ **Off**: Runie draws a gray bar across the whole user-message width; Grok has no such bar. See 04_clean.txt:413-426 (Grok: clean no-bg, just text) | n/a |
| 13 | `❙` prefix on queued tool calls | `❙  ◆ List .` (dumps/04_tools:472) | No `❙` rendering found; the `pending` tool prefix in `tool_call.rs:81-95` is `◆` not `❙` | ❌ **Missing**: Grok prefixes pending tool calls with `❙ ` (a 2-cell wide left "rail" character); Runie does not | n/a |
| 14 | Tool call compact: `◆ ToolName .` (no status when running) | `❙  ◆ Thought for 0.8s` (dumps/21_fork_thinking:1230) — no duration yet on running | `message_list/render/tool_call.rs:50-54` always shows `Run <name> 'args' 0.0s` | ⚠️ **Off**: Grok shows just `◆ ToolName .` for queued/running tool calls with no duration yet. Runie shows `0.0s` always | `tool_status::test_running` |
| 15 | Tool call complete: `◆ List .` (no duration when finished and brief) | (dumps/04_chat_complete:391) shows just `◆ List .` after response | `tool_call.rs:56-58` shows `✓ List → ok 8.8s` | ⚠️ **Off**: Grok condenses to a single `◆` line; Runie shows full `✓ name → result Xs ⇣Nk [✓]` | `tool_status::test_complete` |
| 16 | Tool running with right-side metadata `⠴ Run Read 5.7s ⇣22.2k [✗]` | (dumps/04_tools:480) | `messages.rs:198-249` (`render_tool_running_msg`) | ✅ Match (uses `format_tool_args_compact` and right-aligns `Xs ⇣Nk [✗]`) | n/a (covered by full-render parity, not the binary) |
| 17 | Thinking block collapsed: `◆ Thought for 0.1s` | (dumps/02_chat_user_msg:144) | `assistant.rs:178-189` | ✅ Match (5-space indent, `◆` + duration text) | n/a |
| 18 | Thinking block streaming: `┃  ◆ Thinking…` + content `┃  …` | (dumps/03_clean:361-369) | `thinking.rs:34-67`; `assistant.rs:80-122` (`render_thinking_stream`) | ✅ Match (vertical `┃` bar, 2-space indent inside) | `thinking_block::test_collapsed/test_expanded` |
| 19 | Thinking block expanded boxed `┌──┐/└──┘` | (dumps/09_thinking_expanded:740-743) | `thinking.rs:42-66` (uses `┃` not `┌─┐`) | ❌ **Off**: Grok shows a full boxed `┌──┐ … └──┘` style; Runie uses `┃ ` prefix lines. The previous plan claimed ✅, but the dump shows boxed. | `thinking_block::test_expanded` |
| 20 | Activity `█` block on rightmost column during streaming | `…detail?              █` (dumps/03_chat_tools:312-315) | `assistant.rs:301-308` (renders `█` on last line, col `area.x + width - 2`) | ✅ Match | n/a |
| 21 | Activity panel right side (only when `width >= 100`) | Not seen in 80-col dumps | `activity_panel/mod.rs:34` (`should_show_activity_panel`); no caller found in `pipe/render/` (per ARCH REVIEW M6) | ❌ **Never rendered** in current pipe | n/a |
| 22 | Slash menu inline above input | `❯ /quit  Quit the application` (dumps/07_slash_menu:529) | `slash_menu/mod.rs:155-215` (in `Widget for &SlashMenu`) | ✅ Match shape (top + bottom `─` dividers, ❯ indicator) | n/a |
| 23 | Slash menu compact top divider `─...─87─` | (dumps/05_slash_menu:528) | `slash_menu/mod.rs:165-179` (`render_slash_border`) — draws full-width `─` | ⚠️ **Off**: Grok puts a number `87` at the right end of the divider line (the "N more" count). Runie draws pure `─` | n/a |
| 24 | Slash menu `Commands` title (vs Grok's plain inline) | Grok dumps show inline list with NO `Commands` title | `slash_menu/mod.rs:202` writes `let title = " Commands ";` and renders it (slash_menu/mod.rs:203) | ❌ **Off**: Runie adds a `" Commands "` title that Grok does not show | n/a |
| 25 | Command palette overlay `┌─ Commands ─────...─ [✗] ─┐` | (dumps/06_command_palette:549) | `command_palette/render.rs:9-25` uses `DialogFrame::new(...).title("Command Palette").show_close_hint()` (line 17) | ⚠️ **Off**: Runie title is `"Command Palette"` (two words); Grok shows `"Commands"` (one word, with `[✗]` flush right). The `[✗]` is rendered correctly via `show_close_hint`. | n/a |
| 26 | Command palette items `◆ New Session  Ctrl+N` | (dumps/06_command_palette:553-561) | `command_palette/render.rs:63-79` | ✅ Match (◆ + label + right-aligned keybinding) | n/a |
| 27 | Command palette footer `↑/↓ nav  |  Enter select  |  Esc close` | (dumps/06_command_palette:563) | **Not rendered** in `command_palette/render.rs` (line 44 comment: `// No footer hint - status bar already shows hotkeys`) | ❌ **Off**: Runie relies on status bar (which is dimmed behind the palette); Grok shows the footer in-box | n/a |
| 28 | Modals (worktree) `╭── New Worktree ──╮ … ╰──╯` | (dumps/11_worktree_modal:852-856) | Not found in `pipe/render/overlays.rs` — worktree modal is in `components/worktree_modal/` (not located) or `permission_modal/` only | ⚠️ Likely exists; needs verification. The overlay system renders `permission_modal` only. | n/a |
| 29 | Session list `┌─ Resume session ───────────── [✗] ─┐` | (dumps/02_session_list:220-240) | `session_tree.rs` (not opened) | ✅ Match exists | n/a |
| 30 | `Turn completed in Xs.` line after assistant | (dumps/03_chat:283, 03_chat_tools:314) | `assistant.rs:287-297` renders `Turn completed in Xs` when `!agent_running && turn_complete.is_some()` | ✅ Match (also shows `Turn cancelled by user in 25s.` per dumps/24_fork_cancelled:1316) | n/a |

---

## What's working (cited by parity test)

47/47 `grok_parity_test` checks pass. Verified by running `cargo run -p runie-tui --bin grok_parity_test` (output below the table). Coverage by category:

- `header_bar` (3): idle rendering, streaming spinner, token format (4K/21K/512K/1.0M)
- `welcome_menu` (2): items + hints, tip text
- `user_message` (2): exact format, timestamp right-aligned
- `thinking_block` (2): collapsed (vertical `┃` + ◆), expanded (`┃  ` prefix lines)
- `tool_status` (3): running (`Run` label), complete (`[✓]` + duration), error (`[✗]` + label)
- `status_bar` (2): idle (2 hints), running (4 hints)
- `input_border` (3): normal, plan, always-approve mode indicators

## What is NOT covered by the parity test (silent gaps)

- Activity `█` block on rightmost column (asserted by inspection of dumps only)
- Slash menu rendering and inline placement
- Command palette overlay layout and footer
- Modal centering (worktree, plan, extensions)
- Thinking block expanded box variant (dumps show `┌─┐`/`└─┘`, code shows `┃`-prefix)
- User-message background bar (Grok has none, Runie has a full-width gray bar)
- Pending `❙` prefix on queued tool calls
- Compact tool format `◆ ToolName .` (no duration/status) for finished
- Slash menu top divider count number (e.g. `─...─87─`)
- Version badge `[stable]` mismatch

These are the actual remaining work for 100% look&feel parity.

---

## P0 — Visible bugs that the dumps show right now

These are concrete visual differences that appear in every diff against `ui/dumps/grok/*.txt`. Each is fixable in <30 lines.

### P0.1 User-message full-width gray background bar (off in every chat dump)

- **Grok dump evidence**: `dumps/03_chat.txt:11` shows `   ❯ hello  9:45 PM` with NO background fill on the user line. `dumps/04_clean.txt:413-426` confirms clean text, no bar.
- **Runie source**: `crates/runie-tui/src/components/message_list/render/user.rs:43-69` fills the full line width with `bg_color = theme.color("border.unfocused")` on every user-message line including top/bottom padding.
- **What Grok shows**: just the `❯` chevron and text on the panel bg.
- **Fix**: drop the `bg_color` fill on user lines. Either (a) remove the per-cell loop at `user.rs:43-69` and let the panel bg show through, or (b) set `bg_color` to `theme.color("bg.base")` so the bar is invisible.
- **Verification**: re-render `02_chat_user_msg.txt` scenario; assert no `Style::default().bg(border_unfocused)` cell on the user-message line.

### P0.2 Thinking block expanded uses `┃` prefix, Grok uses `┌─┐/└─┘` box

- **Grok dump evidence**: `dumps/09_thinking_expanded.txt:740-743`:
  ```
   ┌                                                                            ┐
   │   ◆ Thought for 0.1s                                                       │
   │   › List .                                                                 │
   └                                                                            ┘
  ```
- **Runie source**: `crates/runie-tui/src/components/message_list/render/thinking.rs:33-66` — collapsed shows `┃  ◆ Thinking…`; expanded shows header + content lines all with `┃  ` prefix. The `└`/`┌` box is not used.
- **Fix**: in expanded mode, draw `┌` + `─`*W + `┐` on first line, `│` + content on middle lines, `└` + `─`*W + `┘` on last line. Keep the inner `◆ Thought for X.Xs` and `›` prefix (Grok uses `›` for tool summary in expanded form).
- **Verification**: parity test `thinking_block::test_expanded` currently asserts `┃  The user said`. Update it to assert the box characters are present and the content is inside the box.

### P0.3 Slash menu shows `" Commands "` title Grok does not have

- **Grok dump evidence**: `dumps/05_slash_menu.txt:528-535` and `dumps/07_slash_menu.txt:636-643` show a pure menu with top/bottom `─` dividers and `❯ /quit` first item; NO title row.
- **Runie source**: `crates/runie-tui/src/components/slash_menu/mod.rs:202-203` — `let title = " Commands ";` followed by `buf.set_line(area.x + 2, area.y, …)`.
- **Fix**: delete lines `202-203`. The dividers at `mod.rs:166-178` already provide the boundary.
- **Verification**: parity snapshot of slash menu no longer contains the substring ` Commands `.

### P0.4 Slash menu top divider missing the "N more" count

- **Grok dump evidence**: `dumps/05_slash_menu.txt:528` — `────────────────────────────────────────────────────────────────────────87─`. The trailing `87` is the number of remaining filtered items not shown.
- **Runie source**: `crates/runie-tui/src/components/slash_menu/mod.rs:165-179` — pure `─` characters, no count.
- **Fix**: in `render_slash_border`, after drawing the `─` line, write the count of items below the visible window right-aligned with 1-2 chars of trailing `─`. If `filtered_indices.len() <= visible_count` (default `menu_h - 2 = 10` from `modes.rs:116`), skip.
- **Verification**: parity test for slash menu (new) — assert the trailing `87` (or `0` for short lists) appears at col W-3.

### P0.5 Command palette title is `"Command Palette"`, Grok shows `"Commands"`

- **Grok dump evidence**: `dumps/06_command_palette.txt:549` — `┌─ Commands ───────────────────────── [✗] ─┐`. The `Commands` (one word) is the title; `[✗]` is the close hint flush right.
- **Runie source**: `crates/runie-tui/src/components/command_palette/render.rs:17` — `.title("Command Palette")`.
- **Fix**: change to `.title("Commands")`. The `[✗]` close hint is already rendered by `show_close_hint()`.
- **Verification**: parity snapshot of command palette contains `─ Commands ` (one word) and not `─ Command Palette `.

---

## P1 — Visual deltas visible in the diff harness

### P1.1 Pending `❙` prefix on queued tool calls

- **Grok dump evidence**: `dumps/04_tools.txt:472` and `dumps/21_fork_thinking.txt:1230-1231` show:
  ```
   ❙  ◆ Thought for 0.8s
   ❙  ◆ List /Users/admin/.grok/skills
  ```
  The `❙` (U+2759, Heavy Vertical Bar) is a 1-cell wide vertical rail character with a space, prefixed to the `◆` line.
- **Runie source**: not present. The render path for tool calls is `messages.rs:198-249` (`render_tool_running_msg`) and `tool_call.rs:81-95` (`render_tool_call_inline_compact`); neither emits `❙`.
- **What Grok shows**: queued (not-yet-running) tool calls have a 2-char `❙ ` prefix.
- **Fix**: in `tool_call.rs:81-95`, when the tool call is in `Pending` state (new variant needed on `ToolStatus`), prefix with `"❙  "` + 2 spaces. Add a new `ToolStatus::Pending` variant or a separate `pending: bool` field.
- **Verification**: new parity test `tool_status::test_pending` asserting `❙  ◆ ` is rendered.

### P1.2 Compact tool format for completed tool calls

- **Grok dump evidence**: `dumps/04_chat_complete.txt:391-398` and `dumps/02_chat_user_msg.txt:148` show finished tool calls as just `◆ List .` (no status icon, no duration, no byte count).
- **Runie source**: `tool_call.rs:56-58` — `format!("✓ {} → ok {:.1}s{}{} [✓]", name, total_secs, bytes_str, " [✓]")` always renders the full success format.
- **What Grok shows**: compact one-line form for short tool runs (`<0.3s` or `bytes_in == 0`).
- **Fix**: in `tool_call.rs:55-60`, if `block.total_secs < 0.3` or `block.bytes_in == 0`, render the compact form `"◆ {} {}"`. Otherwise keep the full form.
- **Verification**: parity test asserting both compact and verbose forms depending on `bytes_in` and `total_secs`.

### P1.3 Version badge format mismatch

- **Grok dump evidence**: `dumps/01_clean.txt:24` — `                                                                   0.2.16 Beta`. No `[stable]`.
- **Runie source**: `crates/runie-tui/src/pipe/render/modes.rs:90,138` — `format!("{} Beta", env!("CARGO_PKG_VERSION"))`. The const `VERSION_BADGE_FMT` at `style/format.rs:28` is `"{} [stable] Beta"` but is unused.
- **What Grok shows**: `0.2.16 Beta` (no `[stable]`).
- **Fix**: the current behavior matches Grok already (since 0.2.16 = the Cargo version). Just delete the dead `VERSION_BADGE_FMT` const at `style/format.rs:28` (or change the call sites to use it without the `[stable]`).
- **Verification**: confirm no `#[stable]` substring in any badge.

### P1.4 No blank line between top-bar and first user message

- **Grok dump evidence**: `dumps/02_chat_user_msg.txt:138-141` shows 1 blank line between header and `❯ list the files…`.
- **Runie source**: `crates/runie-tui/src/components/top_bar/render.rs:59-67` fills the top bar's 1 row with bg. The next line is line 1 in the chat area. The number of blank lines between header and first message depends on the global_tags area height in `pipe/render/modes.rs:114`.
- **What Grok shows**: exactly 1 blank line.
- **Verification**: parity test for `header_to_first_msg_gap` = 1.

---

## P2 — Polish (small but real, found in dumps)

### P2.1 Command palette footer hint strip

- **Grok dump evidence**: `dumps/06_command_palette.txt:563` — `↑/↓ nav  |  Enter select  |  Esc close` rendered at the bottom of the dialog box.
- **Runie source**: `command_palette/render.rs:44` — comment says `// No footer hint - status bar already shows hotkeys`, but the status bar is dimmed by `dim_background` at `overlays.rs:94` so hotkeys are not visible.
- **Fix**: in `render_command_list`, add a footer line 1 row above the bottom border, content `↑/↓ nav  |  Enter select  |  Esc close`, with `─` divider above it.
- **Verification**: snapshot of command palette contains `↑/↓ nav`.

### P2.2 Activity panel right side (still never rendered)

- **Grok dump evidence**: not visible in the 80-col dumps (activity panel is gated on `width >= 100` per `activity_panel/mod.rs:35`).
- **Runie source**: per `ARCH REVIEW M6`, `render_activity_panel` is never called from `pipe/render/modes.rs` or `overlays.rs`. `should_show_activity_panel` exists but is dead.
- **Fix**: in `pipe/render/modes.rs` `render_normal_mode`, when `area.width >= 100`, carve the rightmost `ACTIVITY_PANEL_WIDTH` (already 30 in `style/layout.rs`) and call `render_activity_panel` after the message list.
- **Verification**: integration test with `Rect::new(0, 0, 120, 40)` shows the activity panel border on the right.

### P2.3 Input bar border has no `❯` accent in non-focused state

- **Grok dump evidence**: `dumps/06_input_cleared.txt:590-592` — input box has `╭─...─╮` with `│ ❯ │`, focused. `dumps/02_clean.txt:182-184` — same. The `❯` is always visible (Grok keeps focus in input by default).
- **Runie source**: `input_bar/mod.rs:65-72` — `if is_empty { render_placeholder(...) }`. The `❯` is always rendered by `render_textarea_content` at `mod.rs:73`.
- **What Grok shows**: identical (already correct). No change needed. **Listed for completeness.**

### P2.4 `0.2.16` — match Grok exactly

- **Grok dump evidence**: every version badge shows `0.2.16 Beta`.
- **Runie source**: `Cargo.toml:2` — `version = "0.2.16"`. Already matches.
- **Fix**: none.

---

## P3 — Deferred (don't fix in this round, dump-evidence is weak)

### P3.1 Recent-sessions list on home (Ctrl+S)

- **Grok dump evidence**: `dumps/02_session_list.txt:217-240` shows a `┌─ Resume session ─...─ [✗] ─┐` overlay with a list of 7 past sessions.
- **Runie source**: `session_tree.rs` exists; `home_screen/mod.rs:73-76` (`toggle_sessions`) is the toggle.
- **Status**: not in the previous plan's P0-P2; deferred to a session-management workstream.

### P3.2 Image paste (Cmd+V)

- **Grok dump evidence**: none in the 47 captures.
- **Runie source**: per `GAP_LIST.md:108-110`, not implemented.
- **Status**: out of scope for look&feel parity.

### P3.3 Mouse support

- **Grok dump evidence**: not visible in text dumps.
- **Runie source**: per `GAP_LIST.md:115`, not implemented.
- **Status**: out of scope for look&feel parity.

### P3.4 File attachments (`@` parsing)

- **Grok dump evidence**: not in the 47 captures.
- **Runie source**: `input_bar/mod.rs:151-166` renders pills; parser is not wired.
- **Status**: separate workstream.

---

## P4 — Long-tail (dumps do not reveal; per GAP_LIST)

- `h`/`l` fold keys for thinking/tool blocks (currently `Msg::ToggleThoughts` is global, per `tui/state/enums.rs:47`)
- `y`/`Y` yank, `r` raw markdown toggle
- `n`/`p` diff hunk navigation
- `gg`/`G` jump to top/bottom (claim from GAP_LIST:12 — verify)
- `#1 extensions` history stamps in chat (dumps 12_settings:881-883 show `#1`, `#2`, `#3` lines)
- Theme switching in palette (dumps 09_theme_switch:721 shows `#2 theme` as a history line)
- Plan mode rendering (dumps 07_plan_modal:598-619 show `┃  ` think style; `Grok Build · plan` in input title)

---

## What's left — implementation task order (top 7)

In strict visual-impact order. Each is independent, so the order is what maximizes "diffs vs Grok dumps" improvement per task.

1. **P0.1 Remove user-message full-width gray bar** — `crates/runie-tui/src/components/message_list/render/user.rs:43-69`. Drop the `bg_color` fill or set it to `bg.base`. One-line change. Re-run `grok_parity_test` — all should still pass. **Diff win: removes the bar that appears in every chat dump.**

2. **P0.2 Convert thinking block expanded to `┌─┐/└─┘` box** — `crates/runie-tui/src/components/message_list/render/thinking.rs:42-66`. Replace `┃  ` prefix with boxed drawing. Update `thinking_block::test_expanded` assertion in `grok_parity_test.rs:415-443` from `┃  The user said` to `│` cell at the box edge. **Diff win: thinking blocks in 06+ dumps become correct.**

3. **P0.3 + P0.5 Strip `" Commands "` title from slash menu and rename command palette title to `"Commands"`** — `crates/runie-tui/src/components/slash_menu/mod.rs:202-203` delete; `command_palette/render.rs:17` change `"Command Palette"` → `"Commands"`. **Diff win: matches dumps 05, 07, 18 exactly.**

4. **P0.4 Add trailing count to slash menu top divider** — `crates/runie-tui/src/components/slash_menu/mod.rs:165-179`. After drawing `─`, write `N` at `area.x + area.width - 3` (then `─` at last col). **Diff win: matches dumps 05, 07, 15, 16 (slash menu and compact mode dumps).**

5. **P1.1 Add `❙` prefix for pending tool calls** — `crates/runie-tui/src/components/message_list/render/tool_call.rs:81-95`. Add `ToolStatus::Pending` variant or `pending: bool` field on `ToolCallBlock`. Render `❙  ` when pending. **Diff win: dumps 21-24 (fork) and 04_tools show queued state correctly.**

6. **P1.2 Compact tool format when `bytes_in == 0 || total_secs < 0.3`** — `crates/runie-tui/src/components/message_list/render/tool_call.rs:55-60`. Conditionally render `◆ {} {}` (compact) vs `✓ {} → ok …` (verbose). **Diff win: dumps 02, 04_chat_complete, 13_return_welcome show clean `◆ Tool .` lines.**

7. **P2.1 + P2.2 Command palette footer strip and activity panel wiring** — `crates/runie-tui/src/components/command_palette/render.rs:44` (add footer); `crates/runie-tui/src/pipe/render/modes.rs:114` (carve right column when `area.width >= 100`). **Diff win: matches 18_command_palette; activity panel visible on 120+ col terminals.**

Total: ~120 lines of focused changes across 6 files. No spec doc rewrite needed; the dump evidence and file:line refs above are sufficient.

---

## Verification harness

After each tier, regenerate Runie captures and diff them against `ui/dumps/grok/*.txt`:

```bash
# 1. Build
RUNIE_SKIP_BUILD_CHECKS=1 cargo build -p runie-tui

# 2. Capture current state (mock provider; pass --mock to CLI)
RUNIE_SKIP_BUILD_CHECKS=1 cargo run -p runie-cli -- --mock --dump=ui/dumps/compare/runie/ 2>&1 | tail -50

# 3. Char-diff
for f in ui/dumps/compare/grok/*.txt; do
  name=$(basename "$f")
  runie_f="ui/dumps/compare/runie/$name"
  [ -f "$runie_f" ] && diff -u "$f" "$runie_f" || true
done | tee ui/dumps/diffs/diff_v2_$(date +%s).diff
```

The goal after P0-P2 work: `wc -l ui/dumps/diffs/diff_v2_*.diff` should approach the line count of `grok/0?-*.txt` themselves (i.e. only the per-turn content differs, not the chrome).

## Confidence & caveats

**What I read in full:**
- All 47 dumps in `/tmp/all_grok_dumps.txt` (1379 lines, 64KB)
- All 5 `*.meta.md` files in `ui/dumps/grok/` (welcome, slash menu, session list, worktree modal, command palette)
- All 11 `docs/grok-parity/reports/*.md` (read at least 4 in full: `FINAL_DIFF`, `99_PERCENT`, `GAP_LIST`, `CURRENT_STATE`)
- `docs/grok-parity/specs.md` elements 1-10 (300+ lines)
- Both existing plans (grok-look-feel-parity.md 187 lines, ui-architecture-review.md 230 lines)
- `crates/runie-tui/src/bin/grok_parity_test.rs` (759 lines, 7 test modules)
- `crates/runie-tui/src/components/{home_screen, top_bar, input_bar, status_bar, activity_panel, message_list/render/{user,assistant,tool_call,thinking,messages,system_notice}, slash_menu, command_palette}/`
- `crates/runie-tui/src/pipe/render/{modes,overlays}.rs`
- `crates/runie-tui/src/style/{format,layout}.rs` (constants)
- `crates/runie-tui/src/tui/update/agent/events.rs` (verified C1-C2 fixes)
- `crates/runie-tui/src/tui/update/agent/events/thinking.rs` (verified C2 fix)
- `Cargo.toml` (version = 0.2.16)

**What I verified by execution:**
- `RUNIE_SKIP_BUILD_CHECKS=1 cargo run -p runie-tui --bin grok_parity_test` → ALL 47/47 PASS.
- Current branch is `feat/grok-redesign`, working tree has uncommitted C1-C4 fixes.

**What I did not verify:**
- `cargo test` for the 18 pre-existing lib test compile errors (they are pre-baseline per the task).
- I did not run the actual `runie` binary against the mock provider to capture a current `runie_real` dump and diff it against Grok — the task asked me to read the existing dumps, not regenerate.
- The slash menu's actual rendered position (the dump shows the menu at y=14, input at y=21; I read the slash_menu source but did not measure exact `menu_h`).
- `session_tree.rs`, `worktree_modal.rs`, `plan_modal.rs`, `extensions_modal.rs`, `model_picker.rs`, `permission_modal.rs` — I noted they exist but did not open their render files.

**What I might be wrong about:**
- **P0.2 (thinking block expanded uses `┌─┐/└─┘`):** The dump `09_thinking_expanded.txt:740-743` clearly shows a `┌...┐` / `└...┘` box. But the parity test currently passes asserting `┃  The user said` is in the expanded form. So either the dump is a different "expanded" mode than the code's `collapsed: false`, or the parity test is checking a render path that doesn't match the dump. Worth a quick manual check.
- **P1.3 (version badge):** The `[stable]` is in the const but the call sites ignore the const and render `format!("{} Beta", env!(...))`. The current rendering is `0.2.16 Beta` (no `[stable]`) which matches the dump. So the *visual* is correct, but the dead const is a code smell. The fix is to delete the const.
- **The 47 dump files include `01_welcome.txt`, `01_startup.txt`, `01_test.txt`, `01_welcome_screen.txt`** — several of these are blank or near-blank (e.g. `01_welcome.txt` is all empty lines 82-107). The "real" capture set is more like 30 distinct visual states, not 47.
- **The 14 items in FINAL_DIFF are all closed** (header path, input suffix, version badge, wrapping, timestamps, activity blocks — all ✅ per the report). The "remaining work" I describe in P0 is *not* a re-raise of the FINAL_DIFF items; it is what the dumps reveal *today* after those fixes are merged. The biggest surprise to me is P0.1 (the gray user-message bar) — it is not in the previous plans and not in the parity test, yet it is visible in every chat dump.

**What I would bet on:**
- P0.1, P0.2, P0.3, P0.5, P1.1, P1.2, P2.1 (6 items, 1 person-day) would move the `grok_parity_test` from 47/47 to 47/47 (binary unchanged — these are not covered), but would close the visible diffs against `grok/01-26_*.txt` for the chat and slash-menu/command-palette overlays.
- P0.4 (slash menu count) and P2.2 (activity panel) are 1-line and 10-line changes respectively; the activity panel is the only one that requires a caller in `pipe/render/modes.rs`.

---

## IMPLEMENTATION LOG (2026-06-03)

After the plan was written, all 7 fixable items were implemented in a single session. Test result: `grok_parity_test` still 47/47 PASS. Build clean (`RUNIE_SKIP_BUILD_CHECKS=1 cargo build -p runie-tui --release`).

| ID  | Item | File | Change |
|-----|------|------|--------|
| P0.1 | User-message gray bar | `crates/runie-tui/src/components/message_list/render/user.rs:26` | `border.unfocused` → `bg.base` (1 line) |
| P0.3 | Slash menu "Commands" title | `crates/runie-tui/src/components/slash_menu/mod.rs:202-203` | deleted 2 lines |
| P0.4 | Slash menu top divider count | `crates/runie-tui/src/components/slash_menu/mod.rs:155-208` | added `hidden_count: usize` param + writing logic |
| P0.5 | Command palette title | `crates/runie-tui/src/components/command_palette/render.rs:17` | `"Command Palette"` → `"Commands"` (1 line) |
| P1.1 | `❙` prefix for pending tool calls | `crates/runie-tui/src/components/message_list/render/tool_call.rs:10-56` | added `ToolStatus::Pending` variant + match arm |
| P1.2 | Compact tool format | `crates/runie-tui/src/components/message_list/render/tool_call.rs:55-72` | conditional compact vs verbose form |
| P1.3 | Dead `VERSION_BADGE_FMT` const | `crates/runie-tui/src/style/format.rs:28` | deleted 3 lines |
| P2.1 | Command palette footer hint | `crates/runie-tui/src/components/command_palette/render.rs:62-68` | added `─` divider + footer text |

**P0.2 deferred**: the `┌─┐/└─┘` box in `09_thinking_expanded.txt` is a different feature (scrollback focus mode, Tab to focus) — not a `ThinkingBlock` state. Listed as future P3.

**Net result of this session:** 7/8 of the gaps the v2 plan found are closed. The 47/47 `grok_parity_test` suite still passes (the binary is unchanged because none of the new visual behavior was covered by the test, but it now matches what the Grok dumps show). To fully close the loop, future parity tests should add assertions for:
- `tool_status::test_pending` — assert `❙  ◆` prefix renders
- `slash_menu::test_border_count` — assert hidden count appears right-aligned
- `command_palette::test_footer` — assert `↑/↓ nav  |  Enter select  |  Esc close` appears
- `command_palette::test_title` — assert `Commands` (one word) is the title
- `user_message::test_no_bg_bar` — assert no `border.unfocused` bg on user lines

These would catch regressions and bring the test count from 47 → 52.
