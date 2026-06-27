# Tasks Directory

**Status**: todo

This directory contains the active task list for Runie development.

## Structure

- `index.json` — Machine-readable task list with status, dependencies, and metadata
- `TEMPLATE.md` — Template for new task files
- `*.md` — Active (todo/in_progress/blocked/cancelled) task definitions
- `archive/` — Completed and superseded tasks (for reference)

## Task Status Values

| Status | Meaning |
|--------|---------|
| `todo` | Not started, ready to work on |
| `in_progress` | Actively being worked on |
| `done` | Completed successfully |
| `blocked` | Blocked by dependencies or architecture issues |
| `cancelled` | Cancelled (YAGNI, wrong assumptions, etc.) |

## Convention

When a task is marked `done` in `index.json`, move its `.md` file to `archive/` in the same commit:

```bash
git mv tasks/<task-id>.md tasks/archive/<task-id>.md
```

When a task is cancelled or blocked with a note, update its `.md` file to document the reason.

This keeps the active task list lean and focused.

## Finding Work

To see active tasks:

```bash
ls tasks/*.md   # All active tasks + TEMPLATE.md + README.md
```

To see blocked vs. unblocked tasks, check `index.json` dependencies.

## Archive

Completed tasks are in `tasks/archive/`. They are kept for historical reference and implementation notes.
