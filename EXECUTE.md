# Execution Loop

The execution loop ("ralph") drives task completion by reading the task index, executing work, and marking progress.

## Loop Rules

1. **Read the index first** — Start by loading `tasks/index.json` to discover available work.

2. **Pick the next task** — Select a task that is:
   - Marked `todo` or `in-progress`
   - Has no unresolved blockers (dependencies satisfied)
   - Belongs to the current milestone priority

3. **Read the task file** — Load `tasks/<id>.md` to understand scope and acceptance criteria.

4. **Implement** — Write code, tests, and documentation needed to satisfy acceptance criteria. Follow project conventions from `AGENTS.md`.

5. **Update status** — Change task status to `done` in the task file when complete.

6. **Commit** — Record progress with atomic commits referencing the task scope.

7. **Repeat** — Return to step 2 until no tasks remain.

## Completion Criteria

A milestone is complete when:
- All tasks in that milestone are marked `done`
- Tests pass for all implemented features
- No regressions in existing functionality
- Documentation reflects current state

## Task Status Values

- `todo` — Not started, available for pickup
- `in-progress` — Currently being worked on
- `blocked` — Has unresolved dependencies
- `done` — Complete, criteria satisfied

## Priority Order

1. Core architecture tasks before feature tasks
2. Dependencies before dependents
3. MVP before R1 before R2 before R3
4. Blocking bugs before new features

## Do Not

- Skip the acceptance criteria review before marking done
- Leave tests failing
- Break existing functionality
- Reference specific task IDs in commit messages (use descriptive scopes instead)
