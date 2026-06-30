# Runie vs Grok Build — Side-by-Side Comparison Plan

## Goal

Run **Grok Build** (xAI terminal coding agent CLI, installed via Homebrew Cask `grok-build`) and **Runie** side-by-side against the same scenarios. Identify dead-ends, confusing flows, errors, and missing features in Runie relative to a reference modern terminal coding agent. Every finding must be tracked as a `tasks/` item and fixed with unit + E2E tests; no manual-only fixes.

## Reference Application

- **Name:** Grok Build
- **Publisher:** xAI
- **Install:** `brew install --cask grok-build`
- **Binary:** `/opt/homebrew/bin/grok` (symlink to `/opt/homebrew/Caskroom/grok-build/0.2.72/grok-0.2.72-macos-aarch64`)
- **Modes:**
  - Interactive TUI: `grok`
  - Headless one-shot: `grok -p "..."`
  - Headless with output formats: `--output-format plain|json|streaming-json`
  - Auto-approve: `--always-approve`
  - Session management: `-s/--session-id`, `-r/--resume`, `-c/--continue`
- **TUI slash commands (reference):** `/model`, `/plan`, `/context`, `/compact`, `/rewind`, `/fork`, `/sessions`, `/usage`, `/always-approve`, `/quit`
- **Authentication:** Browser OAuth or `XAI_API_KEY` env var.

## Current Blockers

1. **macOS quarantine** — the installed binary carries `com.apple.quarantine`, causing `Killed: 9` on launch. Must be removed before any comparison.
2. **Authentication** — Grok Build requires a SuperGrok / X Premium Plus / `XAI_API_KEY`. If no credentials are available, the comparison is blocked and must be documented.
3. **Working-directory safety** — Grok Build may write/edit files. All scenarios must run in a **temporary copy** of the target repo, never in `/Users/admin/Code/GitHub/runie-dev`.

## Preparation Checklist

- [ ] Remove quarantine from `/opt/homebrew/Caskroom/grok-build/0.2.72/grok-0.2.72-macos-aarch64`.
- [ ] Verify `grok --version` or `grok --help` runs without crash.
- [ ] Confirm authentication (interactive login or `XAI_API_KEY`).
- [ ] Create a reusable test harness: `scripts/compare-with-grok-build.sh` that:
  - Makes a temp clone of a fixture repo.
  - Runs the Grok Build command in tmux (TUI) or headless.
  - Runs the equivalent Runie command in tmux/headless.
  - Captures pane output / stdout / stderr.
  - Produces a side-by-side diff report.
- [ ] Choose fixture repos:
  - Small synthetic repo for deterministic assertions.
  - A temp copy of `runie-dev` for realistic codebase scenarios.

## Scenario Matrix

| # | Scenario | Grok Build invocation | Runie invocation | What to compare |
|---|----------|----------------------|------------------|-----------------|
| 1 | **Headless hello** | `grok --no-auto-update -p "hello"` | `runie-headless print "hello"` | Output shape, stop reason, exit code, speed |
| 2 | **Headless file list** | `grok --no-auto-update -p "list files"` | `runie-headless print "list files"` | Tool use, permission handling, final output |
| 3 | **Headless native bash** | `grok --no-auto-update -p "run echo hi"` | `runie-headless print "native tool"` | Permission model, loop behavior, exit code |
| 4 | **Headless edit file** | `grok --no-auto-update -p "add a comment to src/lib.rs"` | `runie-headless print "add a comment to src/lib.rs"` | Edit tool availability, diff output, file mutation |
| 5 | **TUI launch / welcome** | `grok` | `runie` | Welcome text, hint clarity, startup errors |
| 6 | **TUI simple chat** | type "hello" | type "hello" | Response rendering, turn completion, status bar |
| 7 | **TUI slash help** | `/help` | `/help` | Command discoverability, descriptions |
| 8 | **TUI slash model/provider** | `/model`, `/provider` | `/model`, `/provider` | Provider/model switching |
| 9 | **TUI slash sessions** | `/sessions`, `/save`, `/load` | `/sessions`, `/save`, `/load` | Session persistence UX |
| 10 | **TUI slash plan/compact** | `/plan`, `/compact` | `/compact` | Plan mode, context compaction |
| 11 | **TUI tool permission** | prompt that calls bash | `native tool` | Dialog clarity, key handling, always-approve |
| 12 | **TUI file picker / @** | `@src/lib.rs` | `@` | File context attachment |
| 13 | **TUI multi-turn** | two follow-up messages | two follow-up messages | Queue behavior, history continuity |
| 14 | **TUI quit/abort** | `/quit`, Esc, Ctrl+c | `q`, Ctrl+c, Ctrl+s | Safe exit during active turn |
| 15 | **Session resume headless** | `grok --no-auto-update -c -p "what did we do"` | `runie-headless` with session arg | Resume behavior |
| 16 | **Configuration / auth** | `grok login`, `grok inspect` | `runie-headless inspect` | Config discovery, auth setup |
| 17 | **Diff rendering** | prompt producing edits | prompt producing edits | Diff readability, approve/reject flow |
| 18 | **Error recovery** | invalid provider / offline | invalid provider / offline | Error messages, recovery hints |

## Classification of Findings

Each discrepancy must be classified as one of:

- **Missing feature** — Runie does not support a capability Grok Build has.
- **Dead-end** — a Runie flow leads the user to a state with no clear next step.
- **Confusing UX** — Runie supports the scenario but the interaction is unclear.
- **Error / bug** — Runie crashes, loops, or returns an error for a valid scenario.
- **Reference-only** — difference is acceptable; document the design choice.

## Fix Policy

For every non-reference-only finding:

1. Create a focused `tasks/<id>.md` with:
   - Concrete reproduction steps.
   - Expected behavior (matching or explaining the Grok Build reference).
   - Acceptance criteria.
   - Required tests:
     - **Unit tests** — state/logic layer.
     - **E2E tests** — event handling or provider-replay layer.
2. Implement the fix using TDD.
3. Re-run the comparison scenario to verify parity.

No fix is complete without automated tests. No manual-only workarounds.

## Deliverables

1. This plan document.
2. A reusable comparison harness script (`scripts/compare-with-grok-build.sh`).
3. A report document (`docs/superpowers/plans/2026-06-30-runie-vs-grok-build-findings.md`) with the side-by-side results.
4. One or more `tasks/` entries for every actionable finding.

## Blocker Escalation

If Grok Build cannot be launched after quarantine removal or authentication fails:

- File a `tasks/` blocker task: `obtain-grok-build-credentials-for-comparison`.
- Use published Grok Build documentation and screen recordings as a secondary reference.
- Proceed with Runie-only gap analysis against the documented reference behavior.
