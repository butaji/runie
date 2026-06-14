# Consolidate Overlapping Tasks and Fix Task Index

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

## Description

`tasks/index.json` and the `tasks/*.md` files have drifted:

- 13 active tasks lack a `file` field.
- 4 task files are not indexed at all (`fix-model-config-non-determinism.md`,
  `move-grep-find-to-tests.md`, `split-tick-animation.md`,
  `table-driven-command-registration.md`).
- Several task clusters overlap (actor/event, MCP, tool rendering).
- 28 archived task files sit alongside 29 active tasks.

## Acceptance Criteria

- [x] Every active `todo`/`in-progress` task in `tasks/index.json` has a valid `file` field.
- [x] Every `tasks/*.md` file not in `tasks/archive/` is indexed.
- [x] Overlapping tasks are already marked with `depends_on`; the graph is acyclic.
- [x] `tasks/archive/` contains only completed or obsolete tasks.
- [x] `cargo test --workspace` still passes (no code changes).

## Suggested merges

- `inline-tool-rendering` + `tool-call-state-rendering` → keep
  `tool-call-state-rendering` as the leading task.
- `mcp-client-integration` + `mcp-servers-support` → keep
  `mcp-client-integration` as leading; `mcp-servers-support` becomes UI follow-up.
- `actor-runtime-decision`, `event-bus-jsonl-persistence`, `event-subenums` → keep
  separate but document ordering: `actor-runtime-decision` →
  `event-bus-jsonl-persistence` → `event-subenums`.

## Tests

No Rust tests. Verification:

```bash
python3 - <<'PY'
import json, glob, os
idx = json.load(open('tasks/index.json'))
ids = {t['id'] for t in idx['tasks']}
files = {os.path.basename(p)[:-3] for p in glob.glob('tasks/*.md')}
print('unindexed:', files - ids)
print('missing files:', ids - files)
PY
```

## Files touched

- `tasks/index.json`
- Possibly some `tasks/*.md` files (merge notes, add `supersedes`).

## Out of scope

- Actually implementing merged tasks; this is metadata cleanup only.
