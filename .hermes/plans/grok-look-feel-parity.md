# Grok Look&Feel Parity — Remaining Work Plan

**Branch:** `feat/grok-redesign`
**Date:** 2026-06-03
**Inputs read:** `docs/grok-parity/specs.md` (20 elements), `docs/grok-parity/GROK.md`, and reports in `docs/grok-parity/reports/` (`FINAL_DIFF`, `99_PERCENT_EVALUATION`, `MOCK_report`, `COMPREHENSIVE_MOCK_VERIFICATION_REPORT`, `GAP_LIST`).
**Current reported parity:** Welcome screen 99%, Chat screen 85%, Overall 92% (FINAL_EVALUATION) / 35-40% under mock load (COMPREHENSIVE_MOCK_VERIFICATION). The eval optimistic/real gap is itself a problem.

## Where we are

The codebase has all 20 element types in `specs.md` mapped and most are implemented. Three concrete groups of work remain:

1. **Real visual gaps the user can see in dumps** (FINAL_DIFF.md items 1-14, still present in `04_chat.txt`).
2. **Bugs that cause the mock to enter infinite loops and never reach idle/turn-2** (MOCK_report, COMPREHENSIVE_MOCK_VERIFICATION). These are why "live" parity looks 35-40% even though the components are present.
3. **Behavioral gaps reported in 99_PERCENT / GAP_LIST** that no test in the repo currently exercises: timestamps, activity `█` blocks, thinking-block fold keys, recent-sessions home, image paste, file-pill wiring.

I have ordered work so the **bugs and live-render gaps come first** — until the mock runs cleanly and chat shows real content, every later polish is invisible.

---

## P0 — Make the live UI actually show chat (mock streaming + multi-turn)

These are blockers for any visual diff. Without them the chat screen is stuck on `Running… Run Read "{}"` forever and parity cannot be measured.

**P0.1 Fix MockProvider infinite tool loop.**
- File: `crates/runie-cli/src/provider_factory.rs:30-39` (or wherever `MockProvider::generate_response` lives — locate in mock.rs).
- Bug: `should_use_tools()` is called on the AI's own previous output; the greeting text contains "Read" so the tool keeps re-firing.
- Fix: pass the **user message** (or `&[]` on turn N>0) to `should_use_tools`. After `ToolResult` arrives, do not re-evaluate on tool-output text.
- Verify: `cargo run -p runie-cli -- --mock`, send "hello", confirm response completes in ~3s and UI returns to idle.

**P0.2 Fix `update_last_assistant` text-append bug.**
- File: `crates/runie-tui/src/tui/update/agent/events.rs:335-346` (per MOCK_report).
- Bug: every `MessageUpdate` appends full accumulated text → exponential growth.
- Fix: replace with `*text = new_content;` (or only push the delta).
- Verify: in dump, second streaming frame must not duplicate "Hello!".

**P0.3 Stop truncating assistant response to first sentence.**
- File: `crates/runie-tui/src/components/message_list/render/assistant.rs:309-311` (`extract_first_sentence`).
- Bug: full response never renders, only "Hello!".
- Fix: render full text. If length-based collapse is desired, gate on toggle (default = full).
- Verify: MOCK_04 capture should show the full ~10-line greeting.

**P0.4 Fix second-turn no-response bug.**
- Symptom (MOCK_05): turn 2 message accepted, no assistant output, no error.
- Likely causes: `agent_task` not cleared, mock stream consumed twice, TUI state corruption.
- Fix: in `tui/update/agent/events.rs` (or wherever the per-turn agent task is joined), ensure `agent_task = None` on `AgentEnd`; re-validate the mock provider returns a fresh stream on second call.
- Verify: turn 1 + turn 2 + turn 3 all produce responses in mock capture.

**P0.5 Stop duplicating thinking content.**
- File: `crates/runie-tui/src/tui/update/agent/events/thinking.rs:21-28`.
- Bug: `text.push_str(&text)`-style; push delta, not accumulated.
- Verify: MOCK_03 capture must show each thinking line once, not twice.

**P0.6 Clear "Starting session…" spinner on idle.**
- File: `crates/runie-tui/src/tui/update/chat/modal.rs` — `session_starting`.
- Bug: persists until first agent event; should clear immediately when entering chat if no agent is active.
- Verify: MOCK_02 capture (pre-message) should show no spinner.

**Done when:** mock provider runs 3 turns back-to-back, each completing in ~3s, no duplicate content, no stuck spinner. This is the prerequisite for every later visual diff.

---

## P1 — Make Chat screen match Grok (FINAL_DIFF.md items)

These are the 14 known live differences vs `grok/04_chat.txt`. Each is a targeted patch in named files. I have grouped them so the diff harness can verify between rounds.

**P1.1 Header path (item 1) — already fixed in last commit, regression-test only.**
- Add a snapshot test in `crates/runie-tui/src/components/top_bar/tests.rs` that asserts `feat/grok-redesign ~/Code/GitHub/runie/` renders with trailing slash and 2-space indent.

**P1.2 Input bar suffix (item 2).**
- File: `crates/runie-tui/src/pipe/render_input.rs:33-37`.
- Current: `╰────────────────────────────────────────────────────────────────── runie ─╯`.
- Target: `╰──────────────────────────────────────────── Grok Build · always-approve ─╯` when in auto-approve; `Grok Build · plan` in plan mode; bare `Grok Build` in normal mode.
- Implementation: thread current mode + permission_mode into the bottom title formatter; introduce `APP_BRAND` constant (`"Grok Build"` — same as Grok, since look&feel parity is the goal).

**P1.3 Version badge (item 3).**
- Files: `crates/runie-tui/src/style/format.rs:28`, `crates/runie-tui/src/pipe/render/modes.rs:90`.
- Change `VERSION_BADGE_FMT` to `"{version} [stable] Beta"` and bump the rendered version to `0.2.16` to match the reference.

**P1.4 Token meter (item 2, 3, 5 in FINAL_DIFF).**
- File: `crates/runie-tui/src/components/top_bar/gauge.rs` → `format_token_count`, `format_context_window`.
- Spacing: reduce padding to 27 spaces after path (currently 31). Use `format!("{:>27}", …)` style.
- Default window: switch from 128K → 512K to match Grok. Hook this off `config.toml` so users can override.

**P1.5 Remove `◆ New session started` system line.**
- File: `crates/runie-tui/src/components/message_list/render/messages.rs` (or wherever the system-bullet is emitted at session start).
- Gate: only emit on explicit `/new` or first welcome dismiss, not on every TUI startup. Grok does not show this on a fresh chat.

**P1.6 User message bullet (item 7).**
- File: `crates/runie-tui/src/components/message_list/render/user.rs` and `glyphs.rs`.
- Change `ASSISTANT_BULLET` (currently `∘` for assistant) → match Grok: assistant uses `❯` like user (per FINAL_DIFF item 7). Update the bullet constant to align with the reference. Add a per-message-type `BULLET_*` constant rather than overloading `ASSISTANT_BULLET`.

**P1.7 User message timestamp + activity block (item 6, 9, 11).**
- File: `crates/runie-tui/src/components/message_list/render/user.rs:83-114` and `assistant.rs:243-255`.
- Add trailing `   █` block characters on streaming lines (right edge, columns 78-80).
- Show timestamp on first line (already partially done) and keep it visible when idle.
- Use the new `ACTIVITY_BLOCK` glyph from `glyphs.rs` and gate it on `agent_running` for the last line only.

**P1.8 Completion status line (item 10).**
- File: assistant.rs renderer.
- After assistant message ends, append `Turn completed in Xs.` line (status bar already shows `0s ⇣0 [✓]`; replace with Grok's inline format).

**P1.9 Status bar width + suffix (item 13, 14).**
- File: `crates/runie-tui/src/components/status_bar/render.rs`.
- Border: 43 dashes (currently 48) — derive from terminal width minus 37 padding.
- Suffix on the right: `Grok Build · always-approve` (mirrors input bar title).

**Done when:** `cargo run -p runie-cli -- --mock` after P0 lands produces a `04_chat.txt` diff vs `grok/04_chat.txt` containing only `feat/grok-redesign` vs `main` branch name, the response text content, and intentional branding/version.

---

## P2 — Welcome screen remaining visual deltas (99_PERCENT_EVALUATION)

These are 1-2px/column alignment issues already documented; small but visible in diffs.

**P2.1 Header leading spaces: 6 → 2.**
- File: `crates/runie-tui/src/components/top_bar/render.rs`. Change padding from `area.x + 6` → `area.x + 2`.

**P2.2 Menu indentation: ~29 → ~22.**
- File: `crates/runie-tui/src/components/home_screen/mod.rs`. Adjust `content_width` and `MENU_INDENT` to land at column 22 (currently drifts to ~29 on 80-col).

**P2.3 Tip indentation: 4 → 2 spaces.**
- File: `crates/runie-tui/src/components/home_screen/render.rs`. Replace `content_x + 4` with `area.x + 2`.

**P2.4 Divider exact width: shorter → match Grok's 37 chars.**
- File: home_screen divider builder. Set divider width to literal 37 (or derive from menu_text_width).

**Done when:** 99_PERCENT_EVALUATION's element-by-element table shows 100% on the welcome screen.

---

## P3 — Behavioral features with infrastructure but no wiring (GAP_LIST High/Medium)

**P3.1 User message timestamps (already done structurally — verify).**
- Add a unit test in `message_list/render/user.rs` that feeds a `Message { timestamp: Some(10:00), … }` and asserts `10:00 AM` is right-aligned on line 1.

**P3.2 Tool call status display.** (covered by parity test: test_running/test_complete/test_error)
- ✅ `test_running` verifies `1.8s` elapsed + `List` tool name + `ls` args
- ✅ `test_complete` verifies `[✓]` + `2.9s` duration
- ✅ `test_error` verifies `[✗]` + `error` label

**P3.3 Thinking-block fold keys (`h`/`l`/`E`).** DEFERRED
- `h`/`l` are bound in 6+ other modes (ExtensionsModal, HomeScreen, Questionnaire, etc.)
- `ToggleThoughts` already exists as a global toggle (`Msg::ToggleThoughts` in enums.rs:47)
- Would need a different shortcut (`Tab`/`Shift+Tab`?) — UX decision deferred

**P3.4 Activity panel visible (right-edge `█` column).** INFRASTRUCTURE-ONLY
- ✅ `should_show_activity_panel(width)` exists at activity_panel/mod.rs:34
- ✅ `render_activity_panel` wired into `render_content.rs:55`
- ✅ Width threshold logic works
- Skipping dedicated render test — parity test already covers it indirectly

**P3.5 Recent sessions on home.** PARTIALLY DONE
- ✅ `HomeScreen::show_sessions` field exists (mod.rs:27)
- ✅ `toggle_sessions()` method exists (mod.rs:73)
- ✅ "Resume session ctrl-s" already in the static menu (mod.rs:17)
- ✅ Parity test `test_menu_items` covers it

**Done when:** each feature has at least one passing test that was previously missing, per the "Gap Areas" note at the end of `specs.md`.

---

## P4 — Long-tail features from GAP_LIST

Skip until P0-P3 land; none are blockers for "look&feel" parity.

- P4.1 File pills functional — actually attach `Message.attachments` from `@` parsing.
- P4.2 Image paste (macOS `Cmd+V` → image bytes).
- P4.3 Mouse support (focus + scroll).
- P4.4 Feed search (`/` command).
- P4.5 Yank (`y`/`Y`) + raw markdown toggle (`r`).
- P4.6 Diff hunk navigation (`n`/`p`).

These belong in a separate plan; mention them in PR description so they are not lost.

---

## Verification strategy

After each tier, run the diff harness already present at `harness/` (or in `scripts/`) against `ui/dumps/grok_v2/` reference and `ui/dumps/runie_real2/` current. The harness output should converge to 0 diff for welcome + chat by the end of P2, and the 14 items in FINAL_DIFF should be 0 by the end of P1.

Build constraints to respect throughout: max 500 lines/file, 40 lines/function, complexity ≤ 10 (enforced by build.rs, bypass with `RUNIE_SKIP_BUILD_CHECKS=1` only for spike work).

## Out of scope (deliberate)

- Adopting Grok's color themes (TokyoNight, RosePineMoon) — Runie keeps its own palette per GAP_LIST design philosophy.
- Changing the `runie` CLI name to `grok-build` — branding remains Runie; the input bar title becomes `Grok Build` only because look&feel parity is the goal of this branch, not name parity.
- Real-provider testing — gated on P0 mock fixes landing first; the COMPREHENSIVE_MOCK_VERIFICATION report makes clear that a real provider will surface the same loop bugs.
