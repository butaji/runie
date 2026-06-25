# Tasks Directory

This directory contains the active task list for Runie development.

## Structure

- `index.json` — Machine-readable task list with status, dependencies, and metadata
- `TEMPLATE.md` — Template for new task files
- `*.md` — Active (todo/in_progress) task definitions
- `archive/` — Completed and superseded tasks (for reference)

## Convention

When a task is marked `done` in `index.json`, move its `.md` file to `archive/` in the same commit:

```bash
git mv tasks/<task-id>.md tasks/archive/<task-id>.md
```

This keeps the active task list lean and focused.

## Finding Work

To see active tasks:

```bash
ls tasks/*.md   # All active tasks + TEMPLATE.md
```

To see blocked vs. unblocked tasks, check `index.json` dependencies.

## Archive

Completed tasks are in `tasks/archive/`. They are kept for historical reference and implementation notes.
