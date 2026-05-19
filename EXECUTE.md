# Build Anvil — TUI for Agent Swarms

A ratatui terminal app that shows AI coding agents working. Like htop, but for code. Spec (readonly): ANVIL_FINAL_SPEC_v1.md. Progress: TASKS.md

## Phase 1 (now): Skeleton TUI

- Header bar: repo/branch + agent count + $cost/$budget
- Stream: scrollable list of mock entries (thoughts, edits, plans)
- Input: command line at bottom
- Crates: ratatui, crossterm, tokio
- Done when: compiles, runs, shows mock data, quits with ^q

## Phase 2: Live Stream

- Add entry types: ◇ Thought, ◆ Edit, ┌─ Plan, ┌─ Question
- Render diffs inline with line numbers
- Auto-scroll to latest. j/k to navigate. Space to expand.
- Done when: can render 50 mixed entries smoothly

## Phase 3: Command Palette

- &gt; or / opens fuzzy command list
- /spawn, /models, /cost, /pause
- Done when: commands are selectable and executable

## Phase 4: Model Router

- Fetch models.dev API at startup
- Show model selector (Ctrl+L): health dots, cost, latency
- Track spend per model in header
- Done when: can switch between 2+ models, cost updates live

## Phase 5: JS Scripting

- Integrate rquickjs
- Load anvil.js from ~/.anvil/agents/
- Expose ctx: task, router, git, run, ui, human
- Done when: agent pack can override route() and plan()

## Phase 6: Safety + Git

- Safety envelope: cost caps, protected paths, test gate
- Git worktree per agent
- One commit per completed task
- Done when: agent can't overspend or delete .env

## The Ralph Loop

cargo check → fix → cargo check → fix → cargo test → pass → git commit Every
phase must compile and have a visual test before moving to the next.
