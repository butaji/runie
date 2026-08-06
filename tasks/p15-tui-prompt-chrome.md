# p15 — TUI: prompt widget chrome (model caption, multiline indicator, hints, history, placeholder)

**Parity target:** grok prompt widget box.

## Grok reference

`~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/views/prompt_widget/mod.rs`
- Prefix `❯` (line 13 comment: `❯ type here, text wraps ← prefix + TextArea`); prefix can be overridden, e.g. `"! "` in bash mode (line 170-172).
- Border box: top `╭─╮` divider, side `│` borders, bottom `╰─╯` (line 182-186); `show_borders` full vs minimal (border-less but keeps chrome padding).
- `border_color_override` for plan mode (golden) (line 164-168).
- `model_name` rendered (line 311); session title inlined in the top border right-aligned (line 187-189).
- Multiline-mode indicator shown right-aligned when active (line 314).
- Placeholder override (line 173-176), e.g. feedback mode `"Type your feedback..."`.
- Footer hints `Enter` / `Shift+Tab` / `Ctrl+x` (agent_view footer).

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/widgets/prompt.rs`
- Has `❯` prefix, border chrome, model caption, send/mode/shortcut hints (per the earlier visual snapshot work: `top_corner "╭"`, `bottom_corner "╰"`, `model_caption "Grok 4.5"`, `send_hint "Enter"`, `mode_hint "Shift+Tab"`, `shortcut_hint "Ctrl+x"`).

## Adapt to runie

1. Verify the box geometry matches grok: `╭─╮` top, `│` sides, `╰─╯` bottom; `❯` prefix at the cursor column; model caption placement.
2. **Multiline indicator**: show a right-aligned marker when the prompt is multiline (Shift/Alt-Enter produced a newline).
3. **Prompt history**: render the history browse/search overlay (from p14) with the same chrome.
4. **Placeholder**: show placeholder when empty (e.g. `"Type your message..."`), cleared on first edited char (runie already clears welcome on edit).
5. **Model caption**: render the session model name (from `LoopActor` state) in the top border, styled like the bottom info line (grok `prompt_widget/mod.rs:187-189`).
6. **Resize**: border + prefix + caption must reflow correctly at narrow widths (grok handles clipping).

## State machine / variants

Prompt chrome variants:
- Mode: `normal` | `multiline` | `history_browse` | `history_search` | `file_search` | `plan` (golden border).
- Border: `full` (boxed) | `minimal` (border-less, chrome padding only).
- Prefix variants: `❯` (default) | `! ` (bash) | custom override.
- Content: `empty` (placeholder shown) | `editing` (prefix + text).

## Acceptance

- Snapshot tests: full-mode box matches grok chrome (all corners + caption + hints); multiline indicator; empty-vs-editing placeholder; resize reflow.
- `cargo test -p runie-tui` + `--workspace` green.

## Progress

- **In progress (2026-08-05):** Added owned alternate-mode and multiline
  caption indicators while preserving the normal Grok caption. Added a
  deterministic multiline chrome render test and an empty-prompt
  `Type your message...` placeholder. History browsing now renders a
  deterministic `history` caption; search overlays and model-state wiring
  remain. Typing `/history` now enters an owned search state, filters history
  with Up, and renders a `history search` caption.
- **Model projection (2026-08-05):** Prompt captions now read the model name
  from the loop actor's immutable state snapshot, preserving the Grok caption
  fallback when no model is configured. Added a deterministic caption test.
- **Multiline rendering (2026-08-05):** Shift/Alt-Enter input now renders as
  separate prompt rows with one Grok-style gutter prefix on the first row and
  aligned continuation rows; added a focused renderer regression test.
- **YAML scenario (2026-08-05):** The visual YAML runner now accepts
  `Shift+Enter` and `Alt+Enter` steps; `visual-multiline.yaml` exercises the
  multiline chrome and gutter without recompiling the runner.
- **Dynamic layout (2026-08-05):** The prompt region expands with the number
  of logical input lines, so multiline content remains visible instead of
  being clipped by the fixed three-row idle layout.
- **Plan mode (2026-08-05):** Shift+Tab now cycles normal → alternate → plan;
  plan mode renders a gold prompt border and explicit `plan` caption, with
  focused mode and color coverage.

## Completion

Prompt chrome now covers the declared model, placeholder, multiline, history,
file-search/viewer, alternate, and plan variants. Layout height follows logical
content, borders/captions reflow at narrow sizes, and the focused TUI suite is
green.
