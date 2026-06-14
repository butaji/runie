# Vim Navigation Mode

**Status**: done
**Milestone**: R1
**Category**: Input & Commands
**Priority**: P1

## Description

Runie offers an opt-in vim-style feed-navigation mode. The user explicitly
enters nav mode from the input box (via `Esc`), navigates the scrollback
with `j`/`k`/`g`/`G`/`Ctrl+f`/`Ctrl+b`/`/`, and returns to the input box
with `Space` (or `Esc`). This mirrors the common chat-app pattern (Slack,
Discord) where the input is a separate "mode" from the feed.

The status bar hint changes contextually to advertise the available hotkeys
for the current mode (idle / active turn / nav mode / typing).

## What Was Done

- [x] Fixed the orange selection bracket so it matches the *rendered* height of
  a post, including wrapped system/thought messages (the "random bracket"
  regression). The bracket now always forms a `[` shape (`╭` top, `│` body,
  `╰` bottom) that is content + 2 rows tall for every post, including the
  very first post and one-line posts. The renderer builds a row-to-element
  mapping from the actual wrapped lines and overrides the scroll offset in
  nav mode to keep the full bracket visible.
- [x] Replaced the bracket with a thin accent vertical bar (`▎`) in the
  leftmost terminal column for every selected-post row. The bar sits on
  the left edge of the cell and does not steal content space.
- [x] Added a subtle accent-colored background highlight (10% opacity) behind
  the selected post. The highlight spans exactly the same rows as the left
  bar and covers the full terminal width, including outer margins, so there
  are no uncolored gaps.
- [x] Restored the background color for user message posts in the feed. The
  feed renderer applies a `bg.user` background to user message rows, with a
  fallback chain (`bg.elevated` → `bg.panel` → `bg.highlight`) so it works
  across the default theme and third-party themes. The input box chevron
  does **not** inherit this background.
- [x] Added a first-class `Post` DSL in `crates/runie-core/src/ui/posts.rs`
  (`PostBuilder`) so the transform layer declares posts by kind
  (`UserInput`, `AgentResponse`, `Thought`, `ToolRunning`, `ToolDone`, …)
  and expansion state instead of hand-rolling element indices.
- [x] Disabled input box now renders entered text and the cursor with the
  dimmed disabled style (`style_hint` / `style_input_cursor_disabled`) so
  the whole field reads as inactive.
- [x] Added `vim_mode: bool` to `ConfigState` and `config_reload::Config`
  (`[ui] vim_mode = false`).
- [x] Added new events: `Event::GoToTop`, `Event::GoToBottom`,
  `Event::ToggleVimMode`.
- [x] Implemented scroll handlers for `GoToTop` / `GoToBottom` in
  `crates/runie-core/src/update/scroll.rs`.
- [x] Added `AppState::vim_nav_mode: bool` (transient runtime flag,
  distinct from the `config.vim_mode` opt-in).
- [x] Added `AppState::vim_nav_pending: bool` so that two consecutive `Esc`
  presses during an active turn (first = stop the turn, second = enter nav)
  behave as the user expects.
- [x] Wired `Esc` (`Event::DialogBack`) routing in
  `crates/runie-core/src/update/mod.rs`:
  - `Esc` while `vim_nav_mode` → exit nav mode (return to input box).
  - `Esc` while `vim_nav_pending` → enter nav mode.
  - `Esc` while `turn_active` → stop the turn and arm
    `vim_nav_pending` (next `Esc` enters nav).
  - `Esc` while idle and `config.vim_mode` → enter nav mode directly.
  - `Esc` is a no-op when `config.vim_mode` is off.
- [x] Added `handle_vim_nav_event` and `handle_vim_nav_char` in
  `crates/runie-core/src/update/input.rs` so that when `vim_nav_mode` is
  on, `j`/`k`/`g`/`G` are consumed as motions, `Space` exits nav and
  inserts a space, and any other printable char exits nav and is typed.
- [x] Updated `hint_text()` to advertise the current mode:
  - Idle + vim_mode on → `esc nav`.
  - Turn active + vim_mode on → `esc abort·nav`.
  - Nav mode → `j/k scroll · g/G top/bottom · space input · esc input`.
  - Typing → `enter send · alt+enter follow-up · esc clear`.
- [x] `init_ui_config` in `crates/runie-term/src/app_init.rs` now applies
  `[ui] vim_mode` from the config file at startup (the file watcher only
  re-emits provider/model/theme, so this is the only place the initial
  `vim_mode` is wired in).
- [x] `app_init::init_ui_config` is called from `main.rs` after
  `init_truncation`.

## Acceptance Criteria

- [x] `cargo build --workspace` succeeds.
- [x] `cargo test -p runie-core --lib` passes (970 tests).
- [x] `cargo test -p runie-term --lib` passes (118 tests).
- [x] `cargo test -p runie-term --test e2e smoke_ -- --ignored` passes
  (9 smoke tests).
- [x] `cargo test -p runie-core vim` passes (24 tests, including all
  nav-mode tests).
- [x] Manual tmux check confirms: Esc enters nav, j/k/g/G are motions
  (not typed), Space exits nav and inserts a space, typing `a` exits nav
  and inserts `a`, Esc again re-enters nav.

## Tests

### Layer 1 — State/Logic
- [x] `config_vim_mode_default_false`
- [x] `config_vim_mode_true`
- [x] `vim_mode_off_does_not_intercept_j`
- [x] `vim_mode_on_j_scrolls_up`
- [x] `vim_mode_on_k_scrolls_down`
- [x] `vim_mode_on_g_goes_top`
- [x] `vim_mode_on_G_goes_bottom`
- [x] `vim_mode_on_slash_opens_palette`
- [x] `vim_mode_on_y_copies_last_response`
- [x] `vim_mode_printable_char_starts_input`
- [x] `vim_mode_ignored_when_input_not_empty`
- [x] `go_to_top_event_sets_scroll`
- [x] `go_to_bottom_event_clears_scroll`
- [x] `toggle_vim_mode_flips_flag`
- [x] `esc_with_vim_mode_on_and_no_turn_enters_nav_mode`
- [x] `esc_with_vim_mode_off_does_not_enter_nav_mode`
- [x] `esc_during_active_turn_stops_first_then_enters_nav`
- [x] `nav_mode_jk_gg_scroll`
- [x] `space_exits_nav_mode_and_inserts_space`
- [x] `typing_printable_char_exits_nav_and_inserts`
- [x] `nav_mode_off_esc_is_noop`
- [x] `hint_text_shows_esc_nav_when_vim_enabled_and_idle`
- [x] `hint_text_does_not_show_vim_scroll_when_disabled`
- [x] `hint_text_idle_no_turn_mentions_esc_nav`
- [x] `hint_text_without_vim_mode_does_not_mention_nav`
- [x] `hint_text_in_nav_mode_shows_nav_hotkeys`

### Layer 2 — Event Handling
- [x] `plain_escape_emits_dialog_back` — keymap emits `DialogBack` for `Esc`
  so the core can decide abort vs. nav.
- [x] `plain_space_emits_input_space`
- [x] `toggle_vim_mode_event_resolves`
- [x] `event_from_name_go_to_top` / `event_from_name_go_to_bottom`

### Layer 3 — Rendering
- [x] `vim_mode_hint_renders_in_status` (idle hint shows `esc nav`).
- [x] `vim_nav_mode_hint_renders_in_status` (nav hint shows
  `j/k scroll · space input`).
- [x] `vim_mode_scroll_renders_older_content`
- [x] `vim_mode_page_down_renders_newer_content`
- [x] `nav_mode_highlights_selected_post_with_orange_bracket`
- [x] `nav_mode_bracket_matches_wrapped_post_height` (regression guard for
  wrapped system welcome post; now includes a leading spacer so the top
  post also gets a full `[` bracket).
- [x] `nav_mode_bracket_for_one_line_user_post_is_three_rows`.
- [x] `nav_mode_bracket_for_one_line_non_user_post_is_three_rows`.
- [x] `nav_mode_renders_input_box_with_disabled_style`
- [x] Disabled input text and cursor render with dimmed styles.

### Layer 4 — Smoke
- [x] `e2e_shift_enter_inserts_newline` (the existing kitty-protocol CSI
  test still passes alongside the new nav-mode logic).
- [x] All 9 e2e smoke tests in
  `crates/runie-term/tests/e2e/smoke_*.rs` pass with `--ignored`.

> **Note:** Exact-cell nav-mode assertions are covered by the deterministic
> Layer 3 rendering tests above. The previous tmux-based jk-mode cargo tests
> were removed because tmux session lifecycle flakiness made them unreliable
> as part of the automatic suite. The existing rexpect-based smoke tests
> continue to guard against real-binary crashes and event-loop regressions.

## Behaviour reference

```
┌─────────────────────────────────────────────────────────────┐
│ Idle, vim_mode ON, empty input                              │
│   esc nav   ← press Esc → enter nav mode                   │
├─────────────────────────────────────────────────────────────┤
│ Nav mode (j/k/g/G scroll, / palette, y copy)                │
│   j/k scroll · g/G top/bottom · space input · esc input     │
├─────────────────────────────────────────────────────────────┤
│ Turn active, vim_mode ON                                    │
│   enter steer · alt+enter follow-up · esc abort·nav         │
│   (1st Esc = stop turn, 2nd Esc = enter nav)                │
├─────────────────────────────────────────────────────────────┤
│ Typing                                                      │
│   enter send · alt+enter follow-up · esc clear              │
│   (Esc clears the input)                                    │
└─────────────────────────────────────────────────────────────┘
```

## Notes

**Two press semantics for Esc during a turn:** The first `Esc` stops the
agent turn and arms `vim_nav_pending`; the second `Esc` enters nav mode.
This gives the user a deliberate two-step confirmation before they "leave"
the conversation. The pending flag is cleared in `clear_turn_state` so it
cannot linger after a turn ends normally.

**`g` vs `gg`:** Single `g` jumps to the top. A `gg` two-tap chord is not
implemented to avoid a pending-key state machine; can be added later if
needed.

**Out of scope (for this iteration):**
- Visual / selection mode.
- `:` command line.
- `gg` double-tap handling.
- `y`/`p` yank/put of arbitrary text ranges.

## Verification

```bash
cargo build --workspace
cargo test -p runie-core --lib
cargo test -p runie-term --lib
cargo test -p runie-term --test e2e smoke_ -- --ignored
```
