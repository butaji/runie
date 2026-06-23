# Sync docs with code-review findings

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

The architecture and agent-guideline docs need small clarifications based on the recent review: `AGENTS.md` currently claims “Current violations: 0” while the build actually fails, and `docs/Architecture.md` should make the async-IO remediation pattern more concrete.

## Acceptance Criteria

- [ ] `AGENTS.md` no longer contains a manually-maintained “Current violations: 0” claim.
- [ ] `docs/Architecture.md` explains when to use `tokio::fs`, `spawn_blocking`, and `block_in_place_if_runtime`.
- [ ] Doc changes are committed.

## Tests

- N/A — documentation only.

## Files touched

- `AGENTS.md`
- `docs/Architecture.md`

## Implementation

### Step 1: Update `AGENTS.md`

Replace line 147:

```markdown
Current violations: 0
```

with:

```markdown
Violations are detected automatically by `cargo build`; any violation fails the build. Always run `cargo build --workspace` before claiming the codebase is clean.
```

### Step 2: Update `docs/Architecture.md`

After the helper bullet list in the Async IO discipline section, add:

```markdown
Concrete remediation order:
1. If the call site can be made `async`, use `tokio::fs` / `tokio::process`.
2. If the caller is a sync function reached from an async actor (e.g., the update dispatcher or a sync skill hook), wrap the IO in `block_in_place_if_runtime`.
3. If the work is fire-and-forget or long-running, use `spawn_blocking` or `run_blocking_if_runtime`.
```

### Step 3: Commit

```bash
git add AGENTS.md docs/Architecture.md tasks/sync-docs-after-code-review.md tasks/index.json
git commit -m "docs: sync guardrail and async-IO guidance with review findings"
```

## Notes

- No code changes; this task only updates documentation.
