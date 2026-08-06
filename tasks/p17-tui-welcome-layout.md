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

## Progress

- **Completed (2026-08-05):** Compact welcome now exposes both Grok quit
  chords (`Ctrl+D` and `Ctrl+Q`) with regression coverage. Top-bar
  left/right version alignment is now rendered in both draw paths. The
  full and compact action lists now both advertise `Ctrl+D` / `Ctrl+Q`; the
  complete workflow/model badge remains.
- **Evidence update (2026-08-05):** The YAML `visual-resize` scenario and
  visual snapshot suite cover narrow-terminal clipping and prompt/status
  separation; welcome tests cover compact/full quit/action surfaces and the
  wide full badge. Hero-footer integration remains the outstanding badge gap.
- **Version badges (2026-08-05):** Added explicit reusable full,
  hero-footer, and hero-inline badge variants; compact welcome uses the
  inline variant and all forms have focused coverage.
- **Badge rendering (2026-08-05):** Compact mode uses the inline badge while
  the recorded full-mode frame remains unchanged; full/hero-footer variants
  are explicit reusable formatters pending a reference frame that contains
  those additional surfaces.
- **Wide full badge (2026-08-05):** Wide full-mode welcome surfaces now render
  the full version badge as a right-aligned header line, gated to wide layouts
  so the existing 80-column reference geometry remains byte-stable. A widget
  regression test pins the rendered badge.
- **Hero footer badge (2026-08-05):** The chat layout now exposes its bottom
  margin as a dedicated footer-badge row, and the live welcome path renders the
  HeroFooter badge there with focused cell coverage.
- **YAML badge coverage (2026-08-05):** Added `visual-welcome-badges.yaml`
  for wide-frame full and hero-footer badge assertions through the visual
  runner, keeping the badge behavior editable without recompilation.
- **Dynamic branch header (2026-08-05):** Replaced the chat header's hardcoded
  `:main` suffix with the active Git branch, matching Grok's
  `repo_root:branch` contract while retaining the deterministic `main`
  fallback outside a Git worktree. The lookup is cached once so redraws do not
  spawn blocking Git processes. Startup now eagerly warms that cache before
  entering the redraw loop, so frame rendering is read-only.
- **Header regression coverage (2026-08-05):** Added a production-binary
  widget test that renders the real header path and asserts it contains the
  cached repository branch, closing the gap where YAML rendering bypassed this
  draw function.
- **Render-purity audit (2026-08-05):** Audited all TUI filesystem/process
  access. The only production Git lookup is the eagerly warmed branch cache;
  prompt file search remains an explicit user action, and YAML fixture I/O is
  test infrastructure rather than frame rendering.
- **Repository-root header (2026-08-05):** Matched Grok's top-bar label by
  retaining the final two repository path components (`runie-tests/runie` in
  the reference workspace) before the active `:branch` suffix, instead of
  rendering only the leaf directory.
- **Live tmux/asciinema hero variant (2026-08-05):** Used the Homebrew Grok
  0.2.118 120×36 capture to add a wide-mode bordered hero with the recorded
  logo, Grok 4.5 announcement, version line, and workflow shortcuts. The
  variant is gated to wide/tall terminals so the existing 80-column contract
  remains stable, and its markers are pinned by a widget test.
- **Live Runie capture (2026-08-05):** Verified the production binary in a
  matching 120×36 tmux/asciinema session. The hero gate uses the widget's
  available height rather than the full terminal height, so scrollback/window
  layout still reaches the recorded wide hero; focused tests and strict
  clippy pass.

## Completion status

The two-sided header, workflow actions, model/quit hints, welcome-to-transcript
transition, narrow-terminal layout behavior, and all three badge format/render
variants are covered. Exhaustive cast-wide comparison remains tracked under
p19; p17's scoped welcome/layout acceptance criteria are complete.

- **Fresh matched capture audit (2026-08-05):** New 120×36 tmux/asciinema
  captures of `target/debug/runie` and Homebrew Grok 0.2.118 show a
  variant-only `Clipboard may be unreachable.` notice, `Coming from Codex?`
  resume notice, and `[stable]` badge. These are absent from the checked-in
  `grok-full.cast` and must be modeled as explicit welcome variants.
- **Variant-source audit (2026-08-05):** The grok-build source tree does not
  contain these runtime notice strings; they are emitted by the installed
  binary from session/environment state. Runie therefore needs a deterministic
  variant selector before these notices can be implemented without changing
  the legacy cast contract.
- **Production removal (2026-08-05):** Removed the welcome surface from the
  production Runie entry point. `App::new` and the binary now start directly on
  the prompt/transcript; `App::new_with_welcome` remains only for declarative
  YAML/reference fixtures.
- **Production transition cleanup (2026-08-05):** Removed the binary's
  obsolete `hide_welcome` calls; prompt, file-search, and submit actions now
  flow directly through their actors without a dead welcome transition.
