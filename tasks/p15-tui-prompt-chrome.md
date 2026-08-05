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