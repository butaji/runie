# Archive Done Task Files

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Description

`tasks/` currently mixes done, in-progress, and todo task files at the
root. This makes it hard to see what still needs work. Completed tasks
should be moved to `tasks/archive/` while keeping their IDs and history
intact in `tasks/index.json`.

## Acceptance Criteria

- [ ] A `tasks/archive/` directory is created.
- [ ] All `tasks/*.md` files whose `status` is `done` in
  `tasks/index.json` are moved to `tasks/archive/`.
- [ ] `tasks/index.json` remains valid and references the same IDs.
- [ ] No tooling or documentation that reads `tasks/<id>.md` is broken
  (or is updated to check both locations).
- [ ] Active (`todo`/`in-progress`) tasks stay at `tasks/<id>.md`.

## Tests

### Layer 1 — State/Logic
- [ ] Every `id` in `tasks/index.json` has a corresponding `.md` file in
  `tasks/` or `tasks/archive/`.
- [ ] No active task file is in `tasks/archive/`.
- [ ] No done task file remains at `tasks/<id>.md`.

### Layer 2 — Event Handling
- [ ] Not applicable.

### Layer 3 — Rendering
- [ ] Not applicable.

### Layer 4 — Smoke
- [ ] Not applicable.

## Notes

**Cautions:**
- Do not rename files; only move them.
- If any script assumes `tasks/<id>.md`, update it or keep the file as a
  stub that links to the archive.

**Out of scope:**
- Changing task contents.
- Deleting historical tasks.

## Verification

```bash
python3 - <<'PY'
import json, pathlib
idx = json.loads(pathlib.Path('tasks/index.json').read_text())
for t in idx['tasks']:
    p = pathlib.Path(f"tasks/{t['id']}.md")
    a = pathlib.Path(f"tasks/archive/{t['id']}.md")
    assert p.exists() or a.exists(), f"missing {t['id']}"
    if t['status'] in ('todo', 'in-progress'):
        assert p.exists(), f"active task archived: {t['id']}"
print('ok')
PY
```
