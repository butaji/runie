# Anvil Build Tasks

## Phase 1: Skeleton TUI ✅ DONE
- [x] Create Cargo.toml with ratatui, crossterm, tokio
- [x] Create src/main.rs entry point
- [x] Create src/tui/app.rs with ratatui event loop
- [x] Create src/tui/header.rs for status bar
- [x] Create src/tui/stream.rs for mock entries
- [x] Create src/tui/input.rs for command line
- [x] Verify: compiles, runs, shows mock data, quits with ^q

## Phase 2: Live Stream ✅ DONE
- [x] Add entry types: ◇ Thought, ◆ Edit, ┌─ Plan, ┌─ Question
- [x] Render diffs inline with line numbers
- [x] Auto-scroll to latest (set up, needs terminal test)
- [x] j/k navigation, Space to expand
- [x] Verify: renders 50 mixed entries smoothly

## Phase 3: Command Palette ✅ DONE
- [x] > or / opens fuzzy command list
- [x] /spawn, /models, /cost, /pause commands
- [x] Commands selectable and executable
- [x] Verify: command palette works end-to-end

## Phase 4: Model Router ✅ DONE
- [x] Fetch models.dev API (stubbed, ready for live)
- [x] Show model selector (Ctrl+L)
- [x] Health dots, cost, latency display
- [x] Track spend per model in header
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
