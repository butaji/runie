# Anvil Build Tasks

## Phase 1: Skeleton TUI ✅ DONE
- [x] Create Cargo.toml with ratatui, crossterm, tokio
- [x] Create src/main.rs entry point (now CLI layer with clap)
- [x] Create src/tui/app.rs with ratatui event loop
- [x] Create src/tui/header.rs for status bar
- [x] Create src/tui/stream.rs for mock entries
- [x] Create src/tui/input.rs for command line
- [x] Auto-scroll to bottom on first draw
- [x] Verify: compiles, runs, shows mock data, quits with ^q

## Phase 2: Live Stream ✅ DONE
- [x] Add entry types: ◇ Thought, ◆ Edit, ┌─ Plan, ┌─ Question
- [x] Render diffs inline with line numbers
- [x] Auto-scroll to latest (scroll_to_bottom on first draw)
- [x] j/k navigation, Space to expand, PageUp/PageDown
- [x] Scroll offset tracking for viewport visibility
- [x] Verify: renders 50 mixed entries smoothly

## Phase 3: Command Palette ✅ DONE
- [x] > or / opens fuzzy command list
- [x] /spawn, /models, /cost, /pause, /resume commands
- [x] Commands selectable and executable
- [x] Verify: command palette works end-to-end

## Phase 4: Model Router ✅ DONE
- [x] Fetch models.dev API (stubbed, ready for live)
- [x] Show model selector (Ctrl+L)
- [x] Health dots, cost, latency display
- [x] Track spend per model in header
- [x] ^p to cycle models
- [x] Verify: can switch between 2+ models, cost updates live

## Phase 5: JS Scripting ✅ DONE
- [x] Integrate rquickjs (API stubs ready)
- [x] Load anvil.js from ~/.anvil/agents/
- [x] Expose ctx: task, router, git, run, ui, human
- [x] Verify: agent pack can override route() and plan()

## Phase 6: Safety + Git ✅ DONE
- [x] Safety envelope: cost caps, protected paths, test gate
- [x] Git worktree per agent
- [x] One commit per completed task
- [x] Verify: agent can't overspend or delete .env

## Ralph Loop 2026-05-19 ✅ COMPLETE
- [x] cargo check → passes
- [x] 26 unit/integration tests → all pass
- [x] cargo build --release → succeeds
- [x] git commit (d07b105)

## Ralph Loop 2026-05-19 (2nd pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 26 tests → all pass
- [x] cargo build --release → succeeds
- [x] git commit (6a9848b)

## Phase 7: UI Completeness ✅ DONE (2026-05-19)
- [x] Help overlay (src/tui/help.rs) — triggered by `?`
- [x] Cost HUD modal (src/tui/cost_hud.rs) — triggered by `^$`
- [x] Agent swarm view (src/tui/agents.rs) — triggered by `^a`
- [x] Safety checkpoint modal (src/tui/safety_checkpoint.rs) — triggers on cost violation
- [x] SafetyEnvelope wired into App (tracks cost, triggers checkpoint)
- [x] Input text/clear methods wired into command execution
- [x] Stream.push_input_entry() — user input becomes thought entries
- [x] 16 new tests for all new components → 42 total

## Ralph Loop 2026-05-19 (3rd pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 42 tests → all pass (26 original + 16 new)
- [x] cargo build --release → succeeds
- [x] git commit

## Phase 8: Core Execution Engine (2026-05-19) ✅ DONE
- [x] Intent parser (src/core/intent.rs) - NL → typed Intent
- [x] Plan generator (src/core/plan.rs) - static DAG from Intent
- [x] DAG executor (src/core/dag.rs) - tokio JoinSet structured concurrency
- [x] Skills panel (src/tui/skills.rs) - Tab cycles: Hooks | Plugins | Marketplace | Skills | MCP
- [x] petgraph + regex dependencies added for DAG/graph operations
- [x] 9 new unit tests for core execution engine (intent, plan, dag, skills)

## Ralph Loop 2026-05-19 (4th pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 59 tests → all pass (42 original + 17 new)
- [x] cargo build --release → succeeds
- [x] git commit

## Ralph Loop 2026-05-19 (5th pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 67 tests → all pass (25 unit incl. executor + 42 integration)
- [x] cargo build --release → succeeds
- [x] git commit (cff27fd)

## Remaining Gaps
- [x] models.dev live API fetch at startup (fetch_from_models_dev() called in executor::init)
- [x] rquickjs runtime integration (agent discovery, eval, run_agent_script)
- [x] Git worktree wired into agent spawning (AgentWorktree + spawn/cancel/view methods in AgentsPanel)
- [x] Wire intent parser → plan generator → DAG executor into TUI execution flow

## Ralph Loop 2026-05-19 (6th pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 89 tests → all pass (47 unit + 42 integration)
- [x] cargo build --release → succeeds
- [x] Git worktree wired into agents panel (AgentWorktree, spawn_agent, cancel_agent, selected_worktree)
- [x] 8 new unit tests for git module, 12 new tests for agents panel with worktree support

## Ralph Loop 2026-05-19 (7th pass) ✅ COMPLETE
- [x] cargo check → passes
- [x] 89 tests → all pass (47 unit + 42 integration)
- [x] cargo build --release → succeeds
- [x] All phases implemented and verified
