# p17 — TUI: welcome screen + layout/resize parity

**Parity target:** grok welcome screen + overall layout.

## Grok reference

`~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/views/welcome/mod.rs`
- Top bar: `repo_root:branch` (left), version (right) (line 5).
- Version badge variants (line 395-401): full `team | tier | api_key | Grok Build VERSION+channel Beta` (right); hero footer `team | api_key | Grok Build Beta [channel]` (right, gray); hero inline `Grok Build Beta VERSION` (left).
- Quit hints `ctrl+d` / `ctrl+q` (line 48-53).
- `render_version_badge` (line 405) with `xai_grok_version::VERSION` + channel.
- Welcome shows workflow actions (e.g. "New worktree"), model, and hint lines.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/widgets/welcome.rs`
- Welcome modal lines: version/cwd/model block, event-log entry, hint line (per `welcome_modal_lines`).

## Adapt to runie

1. **Top bar**: render `cwd:branch` (left) + version (right) like grok's top bar; runie currently renders a paragraph header `" main ~/Code/GitHub/runie-tests/runie"` — align to the two-sided layout.
2. **Version badge**: render `runie VERSION [channel]` in the three variants (full / hero-footer / hero-inline) matching grok's alignment and color (right-aligned gray).
3. **Quit hints**: show `ctrl+d` and `ctrl+q` on the welcome/idle surface.
4. **Workflow/actions**: render the welcome action list (e.g. "New worktree") and model caption as grok does.
5. **Layout/resize**: dwell on `chat_layout` — verify the regions (header / scrollback / prompt / status) and their constraints match grok's pager layout, and that narrow-terminal clipping is graceful.

## State machine / variants

Welcome surface variants:
- `Hero` (full badge + actions + model) vs `Idle/minimal` (header + prompt only).
- Version badge: `full` | `hero_footer` | `hero_inline`.
- The welcome is replaced by the transcript on first edit/submit (runie clears welcome on edition — retain; grok does the same).

## Acceptance

- Snapshot: welcome screen matches grok's hero layout (top bar left/right, version badge, quit hints, model caption).
- Resize test: at small widths regions don't overlap and text clips gracefully.
- `cargo test -p runie-tui` green.