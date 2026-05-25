# OVERNIGHT AUDIT: TUI CODING AGENT

You have until morning. Do not break the build. Work on a branch `ralph/overnight-audit`.

## 1. UX AUDIT — DEAD-ENDS, INVALID STATES, COGNITIVE LOAD

Audit every screen, modal, and transient state in the TUI. Check these specific failure modes:

| Failure Mode | Check |
|---|---|
| **Dead-ends** | Every modal/dialog must have an escape path (Esc, Back, or Cancel). No state where the only option is to kill the process. |
| **Invalid states** | What happens if: (a) model streams garbage mid-token, (b) network drops during tool call, (c) file is deleted during active edit, (d) API key is invalid but user hits "run", (e) DAG cycle detected, (f) actor panics. Every state must degrade gracefully with a recovery action. |
| **Cognitive load** | Apply Hick's Law and progressive disclosure. If a screen shows &gt;7 active elements, chunk them. Hide advanced config behind "Show advanced". Use consistent keybindings (no `q` to quit on one screen and `q` to queue on another). Status bar must be the single source of truth for system state. |
| **Empty states** | What renders when: no workspace, no models configured, no chat history, no tools available? Empty states must be informative and provide a primary CTA, not a blank box. |
| **Error presentation** | Never dump raw stack traces into the main buffer. Errors get a compact banner + optional "Details" expansion. |
| **Clarity** | All states are presented in the best possible way (minimum text / maximum value) and clear for the user. Any user interaction is having intermediate response and very clear |

**Method:** Walk the code as a state machine. For every `draw()` and `handle_event()`, ask: "What if the user is confused here? What if the data is corrupt?"

## 2. BEHAVIOR VALIDATION — EXPECTED VS ACTUAL

Map the agent loop as a finite state machine. Validate:

- **Transitions:** Every `StateA → StateB` must be defined. If you find an implicit transition, make it explicit or block it.
- **Idempotency:** Re-running the same command twice must not corrupt the workspace or double-spend tokens.
- **Cancellation:** `Ctrl+C` or "Stop" at any point must leave the workspace in a known state (rollback partial file edits, cancel pending API calls).
- **Concurrency:** If multiple actors touch the same file/RPC/model context, verify there is no race window.

**Method:** Add `debug_assert!`s or lightweight tracing for undefined transitions. If a transition isn't in the spec, it's a bug.

## 3. TEST HARNESS — BORROW FROM EXISTING AGENT BENCHMARKS

Integrate evaluation patterns from proven harnesses. Do not reinvent the wheel; adapt these:

- **SWE-bench pattern:** Docker sandbox per task, hidden `test.sh` grader, fail-to-pass + pass-to-pass validation. Add a `cargo test --test agent_harness` that spins up a temp workspace, runs the agent on a micro-task, and verifies the patch. citeweb_search:1#6
- **Terminal Bench 2.0 pattern:** 89-task suite across domains (debugging, ML, Rust). Create a `tasks/` directory with 10 curated micro-tasks (e.g., "add error handling to this function", "refactor module"). Each task has a hidden grader. citeweb_search:1#0
- **bosun-ai/agent-test-harness pattern:** Measure (a) task resolution rate, (b) test coverage added, (c) git diff size, (d) token cost per task. Add a script `scripts/bench_harness.sh` that runs the agent against the task suite and outputs a CSV of these metrics. citeweb_search:1#5
- **harness-bench pattern:** Compare your harness against itself nightly. The script should support `--model` and `--harness-config` flags so you can A/B test prompt changes. citeweb_search:1#2

**Deliverable:** A working `harness/` module with at least 3 end-to-end tasks and a runner script.

## DELIVERABLES

1. `audit/ux_issues.md` — P0/P1/P2 ranked list with file:line references.
2. `audit/behavior_gaps.md` — Undefined state transitions found + proposed fixes.
3. `harness/` — New module with SWE-bench-style sandbox + 3 tasks + runner.
4. `fixes/` — Branch commits, one per logical fix, rebased on main.

## CONSTRAINTS

- No new dependencies without justification.
- All changes must compile with `cargo check --all-targets`.
- If a fix touches the actor loop, add a test proving the state transition is now explicit.
- Do not change the color scheme or visual style unless it directly fixes an invalid state.

Start now. Report progress every 10 tasks or 30 minutes, whichever comes first.


## GIT

Task/scope = commit. Dont push.